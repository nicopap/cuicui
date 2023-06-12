mod impl_fmt;
mod make;
mod minimal;

use std::{mem::size_of, ops::Range};

use datazoo::{
    AssumeSortedByItemExt, EnumMultimap, IndexMap, IndexMultimap, JaggedBitset, SortedByItem,
    SortedIterator,
};
use log::warn;

use crate::binding::{Id, View};
use crate::modify::{Changing, FieldsOf, Indexed, Modify};

pub use minimal::MinResolver;

/// A Resolver for the [`Modify`] trait.
///
/// A resolver stores and manages many
/// different `Modify`, it triggers them when `update` is called and one of the
/// dependencies of the relevant `Modify` has changed.
///
/// The resolver can also read bindings (`Modify` values associated with a plain
/// text name) from the [`View`] struct. `View` tells the `Resolver` which
/// binding was updated since last time `update` was ran.
///
/// When initializing a `Resolver` with `new`, the output should both contain
/// the `Resolver` in question and a list of `Modify` items. Usually, most of
/// the final `Modify::Items` can be created and set at initialization. `Modify`
/// resolved through `Resolver` only manage a small subset of the `Items`.
///
/// # Implementations
///
/// This trait is not sealed, yet it is not recommend to implement it yourself.
/// Here are the provided implementations:
///
/// - `()` empty tuple: This does nothing and can't store `Modify`. This is a
///   dummy implementation for tests and example illustrations.
/// - [`MinResolver`]: A simple resolver that can build an initial `Vec<Modify::Item>`
///   and trigger `Modify` marked as updated in the `View` `update` argument.
/// - [`DepsResolver`]: A fully-featured resolver that manages `Modify`s that
///   may depend on the output of other `Modify`s stored in the same resolver.
///   \
///   `DepsResover` will not only trigger updated modifiers, but also modifiers
///   that depends on updated modifiers (repeating, of course).
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
impl datazoo::index_multimap::Index for ModifyIndex {
    fn get(&self) -> usize {
        self.0 as usize
    }
}
impl From<usize> for ModifyIndex {
    fn from(value: usize) -> Self {
        ModifyIndex::new(value)
    }
}
impl From<u32> for ModifyIndex {
    fn from(value: u32) -> Self {
        ModifyIndex(value)
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

/// A [`Modify`] that apply to a given [`Range`] of [`M::Item`]s in [`M::Items`].
///
/// [`M::Item`]: Modify::Item
/// [`M::Items`]: Modify::Items
#[derive(Debug)]
struct Modifier<M> {
    /// The modifier to apply in the given `range`.
    modify: Option<M>,

    /// The range to which to apply the `modify`.
    range: Range<u32>,
}

/// A fully-featured resolver that manages `Modify`s that
/// may depend on the output of other `Modify`s stored in the same resolver.
///
/// `DepsResover` will not only trigger updated modifiers, but also modifiers
/// that depends on updated modifiers (repeating, of course).
#[derive(Debug)]
pub struct DepsResolver<M: Modify, const MOD_COUNT: usize> {
    modifiers: Box<[Modifier<M>]>,

    /// `Modify` that can be triggered by a `Modify::Field` change.
    /// `f2m` stands for "field-to-modifier dependencies".
    f2m: EnumMultimap<M::Field, ModifyIndex, MOD_COUNT>,

    /// `Modify` that depends on other `Modify`.
    ///
    /// When a `Modify` changes, sometimes, other `Modify` need to run.
    /// `m2m` stands for "modifier-to-modifier dependencies".
    m2m: IndexMultimap<ModifyIndex, ModifyIndex>,

    /// Index in `modifiers` of binding [`Id`].
    /// `b2m` stands for "binding-to-modifier dependencies".
    b2m: IndexMap<Id, ModifyIndex>,

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
    fn index_of(&self, binding: Id) -> Option<ModifyIndex> {
        self.b2m.get(&binding)
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
        for dep_index in self.graph.m2m.get(&index) {
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

        for (&binding, bound_modify) in bindings {
            let Some(mod_index) = self.graph.index_of(binding) else { continue; };

            self.update(mod_index, to_update, ctx, false, Some(bound_modify));
            // TODO(feat): insert modify with dependencies if !modify.depends().is_empty()
        }
        for index in self.graph.depends_on(updated_fields) {
            self.update(index, to_update, ctx, true, None);
        }
    }
}
