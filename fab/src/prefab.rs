use std::fmt;

#[cfg(doc)]
use crate::resolve::Resolver;

use enumset::{EnumSet, EnumSetType};

pub trait Indexed<M: Modify + ?Sized> {
    fn get_mut(&mut self, index: usize) -> Option<&mut M::Item>;
}
impl<T, M: Modify<Item = T>> Indexed<M> for [T] {
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        <[T]>::get_mut(self, index)
    }
}
impl<T, M: Modify<Item = T>> Indexed<M> for Vec<T> {
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        <[T]>::get_mut(self, index)
    }
}

/// Several [`Modify::Field`] of the [`Prefab::Modify`] of `P`.
pub type FieldsOf<M> = EnumSet<<M as Modify>::Field>;

/// A set of operations on `Item`.
///
/// A `Modify` value declares which fields of `Modify::Item` it will [read] and [update].
/// [`Modify::apply`] takes an `Item` and updates its.
/// This allows fine grained update propagation in [`Resolver`].
///
/// `cuicui_fab` provides the [`impl_modify!`] macro to define [`Modify`]
/// more concisely and with less footguns.
///
/// [read]: Modify::depends
/// [update]: Modify::changes
pub trait Modify: Clone + fmt::Debug {
    /// The type on which `Modify` operates
    type Item: Clone + fmt::Debug + Send + Sync;

    /// The underlying [`Self::Item`] storage.
    type Items: Indexed<Self> + Send + Sync;

    /// The [set](EnumSet) of fields that `Self` accesses on `Item`.
    type Field: EnumSetType + fmt::Debug + Send + Sync;

    // TODO(perf): Change detection on context as well.
    /// An additional context **outside of `Item`** that is relevant to operations on `Item`.
    type Context<'a>
    where
        Self: 'a;

    /// Apply this modifier to the [`Self::Item`].
    ///
    /// It is important that `apply`:
    ///
    /// - only reads [`Self::Field`]s returned by [`Self::depends`].
    /// - only updates [`Self::Field`]s returned by [`Self::changes`].
    ///
    /// Otherwise, [`Resolver`] will fail to work properly.
    fn apply(&self, ctx: &Self::Context<'_>, item: &mut Self::Item) -> anyhow::Result<()>;

    /// On what data in [`Self::Item`] does this modifier depends?
    fn depends(&self) -> EnumSet<Self::Field>;

    /// What data in [`Self::Item`] does this `Modify` changes?
    fn changes(&self) -> EnumSet<Self::Field>;
}

/// Holds a [`Prefab::Item`] and keeps track of changes to it.
///
/// You need to use [`Changing::update`] to access the `Item`, this will keep
/// track of the updated fields.
///
/// To reset the field update tracking, use [`Changing::reset_updated`].
pub struct Changing<M: Modify> {
    pub(crate) updated: EnumSet<M::Field>,
    pub(crate) value: M::Item,
}
impl<M: Modify> Changing<M> {
    /// Store this `value` in a `Changing`, with no updated field.
    pub fn new(value: M::Item) -> Self {
        Self { updated: EnumSet::EMPTY, value }
    }
    /// Update `self` with `f`, declaring that `update` is changed.
    ///
    /// If you change fields other than the ones in `updated`, they won't be
    /// tracked as changed. So make sure to properly declare which fields
    /// you are changing.
    pub fn update(&mut self, updated: M::Field, f: impl FnOnce(&mut M::Item)) {
        self.updated |= updated;
        f(&mut self.value);
    }
    /// Reset the change tracker state.
    pub fn reset_updated(&mut self) {
        self.updated = EnumSet::EMPTY;
    }
}
