mod make;

use std::{fmt, iter, ops::Range};

use datazoo::{sorted, BitMultiMap, EnumBitMatrix, EnumMultiMap, SortedIterator};
use log::warn;

use crate::binding::{Id, View};
use crate::prefab::{FieldsOf, Indexed, Modify, Prefab, Tracked};

/// Index in `modifies`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ModifyIndex(u32);
impl fmt::Debug for ModifyIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<M{}>", self.0)
    }
}
impl ModifyIndex {
    fn new(value: usize) -> Self {
        ModifyIndex(value as u32)
    }
}

#[derive(Debug)]
pub enum ModifyKind<P: Prefab> {
    Bound(Id),
    Modify(P::Modifiers),
}
#[derive(Debug)]
pub struct MakeModifier<P: Prefab> {
    pub kind: ModifyKind<P>,
    pub range: Range<u32>,
}

/// A [`ModifyBox`] that apply to a given [`Range`] of [`TextSection`]s on a [`Text`].
struct Modifier<P: Prefab> {
    /// The modifier to apply in the given `range`.
    inner: P::Modifiers,

    /// The range to which to apply the `modify`.
    range: Range<u32>,
}
impl<P: Prefab> fmt::Debug for Modifier<P>
where
    P::Modifiers: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Modifier")
            .field("inner", &self.inner)
            .field("range", &self.range)
            .finish()
    }
}

#[derive(Debug)]
pub struct Resolver<P: Prefab, const MOD_COUNT: usize> {
    modifiers: Box<[Modifier<P>]>,

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
    bindings: sorted::ByKeyBox<Id, Range<u32>>,

    root_mask: EnumBitMatrix<P::Field>,
}

struct Evaluator<'a, P: Prefab, const MC: usize> {
    root: &'a P::Section,
    graph: &'a Resolver<P, MC>,
    ctx: &'a P::Context<'a>,
    to_update: &'a mut P::Sections,
}

impl<P: Prefab, const MC: usize> Resolver<P, MC>
where
    P::Section: Clone + fmt::Debug,
    P::Field: fmt::Debug,
{
    pub fn new(
        modifiers: Vec<MakeModifier<P>>,
        default_section: &P::Section,
        ctx: &P::Context<'_>,
    ) -> (Self, Vec<P::Section>) {
        make::Make::new(modifiers, default_section).build(ctx)
    }
    fn binding_range(&self, binding: Id) -> Option<(usize, Range<u32>)> {
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
        unsafe { self.modifiers.get_unchecked(index.0 as usize) }
    }
    pub fn update<'a>(
        &'a self,
        to_update: &'a mut P::Sections,
        updates: &'a Tracked<P::Section>,
        bindings: View<'a, P>,
        ctx: &'a P::Context<'a>,
    ) {
        let bindings = bindings.changed();
        let Tracked { updated, value } = updates;
        Evaluator { graph: self, ctx, to_update, root: value }.eval(*updated, bindings);
    }
}
impl<'a, P: Prefab, const MC: usize> Evaluator<'a, P, MC>
where
    P::Section: Clone + fmt::Debug,
    P::Field: fmt::Debug,
{
    fn eval_exact(
        &mut self,
        index: ModifyIndex,
        mask: impl SortedIterator<Item = u32>,
        // TODO(clean): flag argument
        uses_root: bool,
    ) -> anyhow::Result<()> {
        let Modifier { inner: modify, range } = self.graph.modify_at(index);

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
        for &dep_index in self.graph.modify_deps.get(&index) {
            if let Err(err) = self.eval_exact(dep_index, mask(), false) {
                warn!("when applying {dep_index:?} child of {index:?}: {err}");
            }
        }
    }
    fn eval<'b>(
        &mut self,
        changes: FieldsOf<P>,
        bindings: impl Iterator<Item = (Id, &'b P::Modifiers)>,
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
            let Modifier { inner: modify, range } = self.graph.modify_at(index);
            let mask = || self.graph.root_mask_for(modify.changes(), range.clone());
            self.eval_with_dependencies(index, mask);
        }
    }
}
