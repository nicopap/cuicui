mod impl_fmt;
mod make;

use std::{mem::size_of, ops::Range};

use datazoo::{
    sorted, AssumeSortedByItemExt, BitMultimap, EnumMultimap, JaggedBitset, SortedByItem,
    SortedIterator,
};
use log::warn;
use smallvec::SmallVec;

use crate::binding::{Id, View};
use crate::prefab::{Changing, FieldsOf, Indexed, Modify, Prefab, PrefabContext, PrefabField};

type SmallKeySorted<K, V, const C: usize> = sorted::KeySorted<SmallVec<[(K, V); C]>, K, V>;

/// Index in `modifies`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ModifyIndex(u32);
impl ModifyIndex {
    fn new(value: usize) -> Self {
        ModifyIndex(value as u32)
    }
}

/// A [`Modify`] either described as a [`Prefab::Modify`] or a binding [`Id`].
pub enum ModifyKind<P: Prefab> {
    Bound {
        binding: Id,
        depends: FieldsOf<P>,
        changes: FieldsOf<P>,
    },
    Modify(P::Modify),
}

/// Describes a [`Modify`] affecting a range of items in the [`Prefab`]
/// and dependency described as [`ModifyKind`].
///
/// Used in [`Resolver::new`] to create a [`Resolver`].
pub struct MakeModify<P: Prefab> {
    pub kind: ModifyKind<P>,
    pub range: Range<u32>,
}
impl<P: Prefab> From<MakeModify<P>> for Modifier<P> {
    fn from(value: MakeModify<P>) -> Self {
        let modify = match value.kind {
            ModifyKind::Bound { .. } => None,
            ModifyKind::Modify(modify) => Some(modify),
        };
        Modifier { modify, range: value.range }
    }
}
impl<P: Prefab> MakeModify<P> {
    fn parent_of(&self, other: &MakeModify<P>) -> bool {
        other.range.end <= self.range.end
    }
    fn depends(&self) -> FieldsOf<P> {
        match &self.kind {
            ModifyKind::Bound { depends, .. } => *depends,
            ModifyKind::Modify(modify) => modify.depends(),
        }
    }
    fn changes(&self) -> FieldsOf<P> {
        match &self.kind {
            ModifyKind::Bound { changes, .. } => *changes,
            ModifyKind::Modify(modify) => modify.changes(),
        }
    }
}

/// A [`ModifyBox`] that apply to a given [`Range`] of [`TextSection`]s on a [`Text`].
struct Modifier<P: Prefab> {
    /// The modifier to apply in the given `range`.
    modify: Option<P::Modify>,

    /// The range to which to apply the `modify`.
    range: Range<u32>,
}

// TODO(clean): Create a trait that wraps `Resolver`, so that we can erase
// MOD_COUNT. We could then use it as an associated type of `Prefab` instead
// of propagating it all the way.
#[derive(Debug)]
pub struct Resolver<P: Prefab, const MOD_COUNT: usize> {
    modifiers: Box<[Modifier<P>]>,

    /// `Modify` that can be triggered by a `PrefabField` change.
    /// `f2m` stands for "field to modifier dependencies".
    f2m: EnumMultimap<PrefabField<P>, ModifyIndex, MOD_COUNT>,

    // TODO(feat): RichText without m2m dependency. This is fairly costly to
    // build and uses several kilobytes of memory.
    /// `Modify` that depends on other `Modify`.
    ///
    /// When a `Modify` changes, sometimes, other `Modify` need to run.
    /// `m2m` stands for "modifier to modifier dependencies".
    m2m: BitMultimap<ModifyIndex, ModifyIndex>,

    /// Index in `modifiers` of binding [`Id`].
    /// `b2m` stands for "binding to modifier dependencies".
    b2m: SmallKeySorted<Id, ModifyIndex, 2>,

    /// Sections **not** to update when a modifier is triggered.
    ///
    /// Each row of the [`JaggedBitset`] corresponds to a [`Modifier`] in
    /// `modifiers`. The row represents the sections to not update.
    masks: JaggedBitset,
}

// TODO(clean): move this after the impl Resolver
struct Evaluator<'a, P: Prefab, const MC: usize> {
    root: &'a P::Item,
    graph: &'a Resolver<P, MC>,
    bindings: View<'a, P>,
}
impl<P: Prefab, const MC: usize> Resolver<P, MC> {
    pub fn new(
        modifiers: Vec<MakeModify<P>>,
        default_section: &P::Item,
        ctx: &PrefabContext<'_, P>,
    ) -> (Self, Vec<P::Item>) {
        assert!(size_of::<usize>() >= size_of::<u32>());

        make::Make::new(modifiers, default_section).build(ctx)
    }
    fn index_of(&self, binding: Id, start_at: usize) -> Option<(usize, ModifyIndex)> {
        let subset = &self.b2m[start_at..];
        let index = start_at + subset.binary_search_by_key(&binding, |d| d.0).ok()?;
        let mod_index = self.b2m[index].1;
        Some((index, mod_index))
    }
    fn depends_on(&self, changes: FieldsOf<P>) -> impl Iterator<Item = ModifyIndex> + '_ {
        self.f2m.all_rows(changes).copied()
    }
    fn modifier_at(&self, index: ModifyIndex) -> &Modifier<P> {
        // SAFETY: we assume that it is not possible to build an invalid `ModifyIndex`.
        // Note: it is only possible to assume this because `ModifyIndex` is not exposed
        // publicly, therefore, the only source of `ModifyIndex` are methods on the very
        // same instance of `Resolver` they are used, and no operation on `Resolver` can
        // invalidate a `ModifyIndex` since all of `Resolver`s slices have fixed size.
        unsafe { self.modifiers.get_unchecked(index.0 as usize) }
    }
    fn masked(&self, index: ModifyIndex) -> impl Iterator<Item = u32> + SortedByItem + '_ {
        // SAFETY: same as above
        unsafe { self.masks.row_unchecked(index.0 as usize) }
    }

    fn modify_at<'a>(
        &'a self,
        index: ModifyIndex,
        overlay: Option<&'a P::Modify>,
    ) -> Option<(&'a P::Modify, impl Iterator<Item = u32> + SortedByItem + 'a)> {
        let (modify, range) = match (self.modifier_at(index), overlay) {
            (Modifier { range, .. }, Some(modify)) => (modify, range),
            (Modifier { modify: Some(modify), range }, None) => (modify, range),
            // `modify: None` yet `overlay: Some(_)`. This happens when a modify
            // coming from a binding has itself a dependency, and that dependency
            // is trying to trigger it.
            // At construction time, `modify` is set to None. Only when bound
            // (`bindings` parameter of `update`) will this `modify` be set to
            // `Some(_)`. We can't do anything with this yet, so we skip.
            (Modifier { modify: None, .. }, None) => return None,
        };
        let range = range.clone();
        // note: the check skips reading `masks` when we know there can't be one.
        let mask = (range.len() > 1)
            .then(|| self.masked(index))
            .into_iter()
            .flatten()
            .map(move |i| i + range.start)
            .assume_sorted_by_item();
        Some((modify, range.difference(mask)))
    }
    pub fn update<'a>(
        &'a self,
        to_update: &mut P::Items,
        updates: &'a Changing<P::Modify>,
        bindings: View<'a, P>,
        ctx: &PrefabContext<P>,
    ) {
        let Changing { updated, value } = updates;
        Evaluator { graph: self, root: value, bindings }.update_all(*updated, to_update, ctx);
    }
}

impl<'a, P: Prefab, const MC: usize> Evaluator<'a, P, MC> {
    // TODO(clean): flag arguments are icky
    fn update(
        &self,
        index: ModifyIndex,
        to_update: &mut P::Items,
        ctx: &PrefabContext<P>,
        field_depends: bool,
        overlay: Option<&P::Modify>,
    ) {
        let Some((modify, range)) = self.graph.modify_at(index, overlay) else { return; };
        for section in range {
            let section = to_update.get_mut(section as usize).unwrap();
            if field_depends {
                *section = self.root.clone();
            }
            if let Err(error) = modify.apply(ctx, section) {
                warn!("Error when applying modifier {index:?} {modify:?}: {error}");
            };
        }
        for &dep_index in self.graph.m2m.get(&index) {
            self.update(dep_index, to_update, ctx, false, None);
        }
    }
    fn update_all(
        &self,
        updated_fields: FieldsOf<P>,
        to_update: &mut P::Items,
        ctx: &PrefabContext<P>,
    ) {
        let bindings = self.bindings.changed();

        let mut lookup_start = 0;
        for (&binding, bound_modify) in bindings {
            let Some(ret) = self.graph.index_of(binding, lookup_start) else { continue; };
            let (index, mod_index) = ret;

            lookup_start = index + 1;

            self.update(mod_index, to_update, ctx, false, Some(bound_modify));
            // TODO(feat): insert modify with dependencies if !modify.depends().is_empty()
        }
        for index in self.graph.depends_on(updated_fields) {
            self.update(index, to_update, ctx, true, None);
        }
    }
}
