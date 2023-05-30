mod impl_fmt;
mod make;

use std::{fmt, iter, ops::Range};

use datazoo::SortedPairIterator;
use datazoo::{
    enumbitmatrix::Rows, sorted, BitMultiMap, EnumBitMatrix, EnumMultiMap, SortedIterator,
};
use log::warn;
use smallvec::SmallVec;

use crate::binding::{Id, View};
use crate::prefab::{Changing, FieldsOf, Indexed, Modify, Prefab, PrefabContext, PrefabField};

pub type SmallSortedByKey<K, V, const C: usize> = sorted::KeySorted<SmallVec<[(K, V); C]>, K, V>;

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

#[derive(Debug)]
pub struct Resolver<P: Prefab, const MOD_COUNT: usize> {
    modifiers: Box<[Modifier<P>]>,

    /// `Modify` that can be triggered by a `PrefabField` change.
    /// `f2m` stands for "field to modifier dependencies".
    f2m: EnumMultiMap<PrefabField<P>, ModifyIndex, MOD_COUNT>,

    // TODO(feat): RichText without m2m dependency. This is fairly costly to
    // build and uses several kilobytes of memory.
    /// `Modify` that depends on other `Modify`.
    ///
    /// When a `Modify` changes, sometimes, other `Modify` need to run.
    /// `m2m` stands for "modifier to modifier dependencies".
    m2m: BitMultiMap<ModifyIndex, ModifyIndex>,

    /// Index in `modifiers` of binding [`Id`].
    /// `b2m` stands for "binding to modifier dependencies".
    b2m: SmallSortedByKey<Id, ModifyIndex, 2>,

    /// Sections **not** to update for a given field when a `f2m` dependency  is triggered.
    root_mask: EnumBitMatrix<PrefabField<P>>,
}

struct Evaluator<'a, P: Prefab, const MC: usize> {
    root: &'a P::Item,
    graph: &'a Resolver<P, MC>,
    ctx: &'a PrefabContext<'a, P>,
    to_update: &'a mut P::Items,
}

impl<P: Prefab, const MC: usize> Resolver<P, MC>
where
    P::Item: Clone + fmt::Debug,
    PrefabField<P>: fmt::Debug,
{
    pub fn new(
        modifiers: Vec<MakeModify<P>>,
        default_section: &P::Item,
        ctx: &PrefabContext<'_, P>,
    ) -> (Self, Vec<P::Item>) {
        make::Make::new(modifiers, default_section).build(ctx)
    }
    fn binding_range(&self, start_at: usize, binding: Id) -> Option<(usize, Range<u32>)> {
        let subset = &self.b2m[start_at..];
        let index = start_at + subset.binary_search_by_key(&binding, |d| d.0).ok()?;
        let mod_index = self.b2m[index].1;
        let mod_range = self.modify_at(mod_index).range.clone();
        Some((index, mod_range))
    }
    fn change_modifies(&self, changes: FieldsOf<P>) -> impl Iterator<Item = ModifyIndex> + '_ {
        self.f2m.all_rows(changes).copied()
    }
    fn root_mask_for(&self, changes: FieldsOf<P>, range: Range<u32>) -> Rows<PrefabField<P>> {
        self.root_mask.rows(changes, range)
    }
    fn modify_at(&self, index: ModifyIndex) -> &Modifier<P> {
        // SAFETY: we kinda assume that it is not possible to build an invalid `ModifyIndex`.
        unsafe { self.modifiers.get_unchecked(index.0 as usize) }
    }
    pub fn update<'a>(
        &'a self,
        to_update: &'a mut P::Items,
        updates: &'a Changing<P::Item, P::Modify>,
        bindings: View<'a, P>,
        ctx: &'a PrefabContext<'a, P>,
    ) {
        let bindings = bindings.changed();
        let Changing { updated, value } = updates;
        Evaluator { graph: self, ctx, to_update, root: value }.eval(*updated, bindings);
    }
}
impl<'a, P: Prefab, const MC: usize> Evaluator<'a, P, MC>
where
    P::Item: Clone + fmt::Debug,
    PrefabField<P>: fmt::Debug,
{
    fn eval_exact(
        &mut self,
        index: ModifyIndex,
        mask: impl SortedIterator<Item = u32>,
        // TODO(clean): flag argument
        uses_root: bool,
    ) -> anyhow::Result<()> {
        let Modifier { modify: Some(modify), range } = self.graph.modify_at(index) else {
            return Ok(());
        };

        for section in range.clone().difference(mask) {
            let section = self.to_update.get_mut(section as usize).unwrap();
            if uses_root {
                *section = self.root.clone();
            }
            modify.apply(self.ctx, section)?;
        }
        Ok(())
    }
    fn eval_with_dependencies<I>(&mut self, index: ModifyIndex, mask: impl Fn() -> I)
    where
        I: SortedIterator<Item = u32>,
    {
        if let Err(err) = self.eval_exact(index, mask(), true) {
            warn!("when applying {index:?}: {err}");
        }
        for &dep_index in self.graph.m2m.get(&index) {
            if let Err(err) = self.eval_exact(dep_index, mask(), false) {
                warn!("when applying {dep_index:?} child of {index:?}: {err}");
            }
        }
    }
    fn eval<'b>(
        &mut self,
        changes: FieldsOf<P>,
        bindings: impl SortedPairIterator<&'b Id, &'b P::Modify, Item = (&'b Id, &'b P::Modify)>,
    ) where
        P::Modify: 'b,
    {
        let mut last_index = 0;
        for (id, modify) in bindings {
            let Some((index, range)) = self.graph.binding_range(last_index, *id) else {
                continue;
            };
            last_index = index + 1;
            for section in range.difference(iter::empty()) {
                let idx = section as usize;
                let section = self.to_update.get_mut(idx).expect("Section within range");
                if let Err(err) = modify.apply(self.ctx, section) {
                    warn!("when applying {id:?}: {err}");
                }
                // TODO(feat): insert modify with dependencies if !modify.depends().is_empty()
            }
            // TODO(bug): we aren't updating modifiers that depends on bindings
        }
        for index in self.graph.change_modifies(changes) {
            let Modifier { modify: Some(modify), range } = self.graph.modify_at(index) else {
                continue;
            };
            let mask = || self.graph.root_mask_for(modify.changes(), range.clone());
            self.eval_with_dependencies(index, mask);
        }
    }
}
