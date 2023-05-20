use std::{fmt, iter, ops::Range};

use bevy_log::warn;
use datazoo::{sorted, BitMultiMap, EnumBitMatrix, EnumMultiMap, SortedIterator};

use crate::binding::{BindingId, BindingsView};
use crate::prefab::{FieldsOf, Modify, Prefab, Sequence, Tracked};

/// Index in `modifies`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ModifyIndex(u32);
impl fmt::Debug for ModifyIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<M{}>", self.0)
    }
}

/// A [`ModifyBox`] that apply to a given [`Range`] of [`TextSection`]s on a [`Text`].
#[derive(Debug)]
struct Modifier<P: Prefab> {
    /// The modifier to apply in the given `range`.
    modify: P::Modifiers,

    /// The range to which to apply the `modify`.
    range: Range<u32>,
}

#[derive(Debug)]
pub struct Resolver<P: Prefab, const MOD_COUNT: usize> {
    modifies: Box<[Modifier<P>]>,

    /// `Modify` that can be triggered by a context change
    direct_deps: EnumMultiMap<P::Field, ModifyIndex, MOD_COUNT>,

    // TODO(feat): RichText without m2m dependency. This is fairly costly to
    // build and uses several kilobytes of memory.
    /// `Modify` that depends on other `Modify`.
    ///
    /// When a `Modify` changes, sometimes, other `Modify` need to run.
    modify_deps: BitMultiMap<ModifyIndex, ModifyIndex>,

    // TODO(feat): Multiple bindings, see `nested_modify.md#bindings-representation`
    /// Binding ranges.
    ///
    /// Note that this prevents having 1 binding to N instances.
    bindings: sorted::ByKeyBox<BindingId, Range<u32>>,

    root_mask: EnumBitMatrix<P::Field>,
}

struct ConcreteResolver<'a, P: Prefab, const MC: usize> {
    root: &'a P,
    graph: &'a Resolver<P, MC>,
    ctx: &'a P::Context,
    to_update: &'a mut P::Collection,
}

impl<P: Prefab, const MC: usize> Resolver<P, MC>
where
    P: Clone,
{
    fn binding_range(&self, binding: BindingId) -> Option<(usize, Range<u32>)> {
        // TODO(perf): binary search THE FIRST binding, then `intersected`
        // the slice from it to end of `dynamic` with the sorted Iterator of BindingId.
        let index = self.bindings.binary_search_by_key(&binding, |d| d.0).ok()?;
        Some((index, self.bindings[index].1.clone()))
    }
    fn change_modifies(&self, changes: FieldsOf<P>) -> impl Iterator<Item = ModifyIndex> + '_ {
        self.direct_deps.all_rows(changes).copied()
    }
    fn root_mask_for(
        &self,
        _changes: FieldsOf<P>,
        _range: Range<u32>,
    ) -> impl SortedIterator<Item = u32> + '_ {
        // TODO(bug): need to get all change masks and merge them
        iter::empty()
    }
    fn modify_at(&self, index: ModifyIndex) -> &Modifier<P> {
        // SAFETY: we kinda assume that it is not possible to build an invalid `ModifyIndex`.
        unsafe { self.modifies.get_unchecked(index.0 as usize) }
    }
    pub fn update<'a>(
        &'a self,
        to_update: &'a mut P::Collection,
        updates: &'a Tracked<P>,
        bindings: BindingsView<'a, P>,
        ctx: &'a P::Context,
    ) {
        let bindings = bindings.changed();
        let Tracked { updated, value } = updates;
        ConcreteResolver { graph: self, ctx, to_update, root: value }.update(*updated, bindings);
    }
}
impl<'a, P: Prefab, const MC: usize> ConcreteResolver<'a, P, MC>
where
    P: Clone,
{
    fn apply_modify(
        &mut self,
        index: ModifyIndex,
        mask: impl SortedIterator<Item = u32>,
        // TODO(clean): flag argument
        uses_root: bool,
    ) -> anyhow::Result<()> {
        let Modifier { modify, range } = self.graph.modify_at(index);

        for section in range.clone().difference(mask) {
            let section = self.to_update.get_mut(section as usize).unwrap();
            if uses_root {
                *section = self.root.clone();
            }
            modify.apply(self.ctx, section)?;
        }
        Ok(())
    }
    fn apply_modify_deps<I>(&mut self, index: ModifyIndex, mask: impl Fn() -> I)
    where
        I: SortedIterator<Item = u32>,
    {
        if let Err(err) = self.apply_modify(index, mask(), true) {
            warn!("when applying {index:?}: {err}");
        }
        for &dep_index in self.graph.modify_deps.get(&index) {
            if let Err(err) = self.apply_modify(dep_index, mask(), false) {
                warn!("when applying {dep_index:?} child of {index:?}: {err}");
            }
        }
    }
    pub fn update<'b>(
        &mut self,
        changes: FieldsOf<P>,
        bindings: impl Iterator<Item = (BindingId, &'b P::Modifiers)>,
    ) where
        P::Modifiers: 'b,
    {
        for (id, modify) in bindings {
            let Some((_, range)) = self.graph.binding_range(id) else {
                continue;
            };
            for section in range.difference(iter::empty()) {
                let section = self.to_update.get_mut(section as usize).unwrap();
                if let Err(err) = modify.apply(self.ctx, section) {
                    warn!("when applying {id:?}: {err}");
                }
            }
        }
        for index in self.graph.change_modifies(changes) {
            let Modifier { modify, range } = self.graph.modify_at(index);
            let mask = || self.graph.root_mask_for(modify.changes(), range.clone());
            self.apply_modify_deps(index, mask);
        }
    }
}
