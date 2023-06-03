mod impl_fmt;
mod make;
mod minimal;

use std::{mem::size_of, ops::Range};

use datazoo::{
    sorted, AssumeSortedByItemExt, BitMultimap, EnumMultimap, JaggedBitset, SortedByItem,
    SortedIterator,
};
use log::warn;
use smallvec::SmallVec;

use crate::binding::{Id, View};
use crate::modify::{Changing, FieldsOf, Indexed, Modify};

pub use minimal::MinResolver;

type SmallKeySorted<K, V, const C: usize> = sorted::KeySorted<SmallVec<[(K, V); C]>, K, V>;

pub trait Resolver<M: Modify>: Sized {
    fn new(
        modifiers: Vec<MakeModify<M>>,
        default_section: &M::Item,
        ctx: &M::Context<'_>,
    ) -> (Self, Vec<M::Item>);

    fn update<'a>(
        &'a self,
        to_update: &mut M::Items,
        updates: &'a Changing<M>,
        bindings: View<'a, M>,
        ctx: &M::Context<'_>,
    );
}

/// Index in `modifies`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ModifyIndex(u32);
impl ModifyIndex {
    fn new(value: usize) -> Self {
        ModifyIndex(value as u32)
    }
}

/// A [`Modify`] either described as `M` or a binding [`Id`].
pub enum ModifyKind<M: Modify> {
    Bound {
        binding: Id,
        depends: FieldsOf<M>,
        changes: FieldsOf<M>,
    },
    Modify(M),
}

/// Describes a [`Modify`] affecting a range of items in the [`Modify::Items`]
/// and dependency described as [`ModifyKind`].
///
/// Used in [`Resolver::new`] to create a [`Resolver`].
pub struct MakeModify<M: Modify> {
    pub kind: ModifyKind<M>,
    pub range: Range<u32>,
}
impl<M: Modify> From<MakeModify<M>> for Modifier<M> {
    fn from(value: MakeModify<M>) -> Self {
        let modify = match value.kind {
            ModifyKind::Bound { .. } => None,
            ModifyKind::Modify(modify) => Some(modify),
        };
        Modifier { modify, range: value.range }
    }
}
impl<M: Modify> MakeModify<M> {
    fn parent_of(&self, other: &MakeModify<M>) -> bool {
        other.range.end <= self.range.end
    }
    fn depends(&self) -> FieldsOf<M> {
        match &self.kind {
            ModifyKind::Bound { depends, .. } => *depends,
            ModifyKind::Modify(modify) => modify.depends(),
        }
    }
    fn changes(&self) -> FieldsOf<M> {
        match &self.kind {
            ModifyKind::Bound { changes, .. } => *changes,
            ModifyKind::Modify(modify) => modify.changes(),
        }
    }
}

/// A [`ModifyBox`] that apply to a given [`Range`] of [`TextSection`]s on a [`Text`].
#[derive(Debug)]
struct Modifier<M> {
    /// The modifier to apply in the given `range`.
    modify: Option<M>,

    /// The range to which to apply the `modify`.
    range: Range<u32>,
}

// TODO(clean): Create a trait that wraps `Resolver`, so that we can erase
// MOD_COUNT. We could then use it as an associated type of `Modify` instead
// of propagating it all the way.
#[derive(Debug)]
pub struct DepsResolver<M: Modify, const MOD_COUNT: usize> {
    modifiers: Box<[Modifier<M>]>,

    /// `Modify` that can be triggered by a `Modify::Field` change.
    /// `f2m` stands for "field to modifier dependencies".
    f2m: EnumMultimap<M::Field, ModifyIndex, MOD_COUNT>,

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

impl<M: Modify, const MC: usize> Resolver<M> for DepsResolver<M, MC> {
    fn new(
        modifiers: Vec<MakeModify<M>>,
        default_section: &M::Item,
        ctx: &M::Context<'_>,
    ) -> (Self, Vec<M::Item>) {
        assert!(size_of::<usize>() >= size_of::<u32>());

        make::Make::new(modifiers, default_section).build(ctx)
    }
    fn update<'a>(
        &'a self,
        to_update: &mut M::Items,
        updates: &'a Changing<M>,
        bindings: View<'a, M>,
        ctx: &M::Context<'_>,
    ) {
        let Changing { updated, value } = updates;
        Evaluator { graph: self, root: value, bindings }.update_all(*updated, to_update, ctx);
    }
}
impl<M: Modify, const MC: usize> DepsResolver<M, MC> {
    fn index_of(&self, binding: Id, start_at: usize) -> Option<(usize, ModifyIndex)> {
        let subset = &self.b2m[start_at..];
        let index = start_at + subset.binary_search_by_key(&binding, |d| d.0).ok()?;
        let mod_index = self.b2m[index].1;
        Some((index, mod_index))
    }
    fn depends_on(&self, changes: FieldsOf<M>) -> impl Iterator<Item = ModifyIndex> + '_ {
        self.f2m.all_rows(changes).copied()
    }
    fn modifier_at(&self, index: ModifyIndex) -> &Modifier<M> {
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
        overlay: Option<&'a M>,
    ) -> Option<(&'a M, impl Iterator<Item = u32> + SortedByItem + 'a)> {
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
}

struct Evaluator<'a, M: Modify, const MC: usize> {
    root: &'a M::Item,
    graph: &'a DepsResolver<M, MC>,
    bindings: View<'a, M>,
}
impl<'a, M: Modify, const MC: usize> Evaluator<'a, M, MC> {
    // TODO(clean): flag arguments are icky
    fn update(
        &self,
        index: ModifyIndex,
        to_update: &mut M::Items,
        ctx: &M::Context<'_>,
        field_depends: bool,
        overlay: Option<&M>,
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
        updated_fields: FieldsOf<M>,
        to_update: &mut M::Items,
        ctx: &M::Context<'_>,
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
