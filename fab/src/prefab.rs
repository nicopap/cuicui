use std::fmt;

#[cfg(doc)]
use crate::resolve::Resolver;

use enumset::{EnumSet, EnumSetType};

pub trait Indexed<P: Prefab + ?Sized> {
    fn get_mut(&mut self, index: usize) -> Option<&mut P::Item>;
}
impl<T, P: Prefab<Item = T>> Indexed<P> for [T] {
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        <[T]>::get_mut(self, index)
    }
}
impl<T, P: Prefab<Item = T>> Indexed<P> for Vec<T> {
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        <[T]>::get_mut(self, index)
    }
}

/// The [`Modify::Field`] of the [`Prefab::Modify`] of `P`.
pub type PrefabField<P> = Key<<P as Prefab>::Modify>;
type Key<M> = <M as Modify>::Field;

/// Several [`Modify::Field`] of the [`Prefab::Modify`] of `P`.
pub type FieldsOf<P> = EnumSet<PrefabField<P>>;

/// The [`Modify::Context`] of the [`Prefab::Modify`] of `P`.
pub type PrefabContext<'a, P> = Ctx<'a, <P as Prefab>::Modify>;
type Ctx<'a, M> = <M as Modify>::Context<'a>;

/// A set of operations on `I`.
///
/// A type `T` that implements `Modify<I>` represents a set of operations
/// possible on `I` — an `item`.
///
/// A `Modify` value declares which fields of `I` it will [read] and [update].
/// [`Modify::apply`] takes an `I` and updates its.
/// This allows fine grained update propagation in [`Resolver`].
///
/// `cuicui_fab` provides the [`impl_modify!`] macro to define [`Modify`]
/// more concisely and with less footguns.
///
/// [read]: Modify::depends
/// [update]: Modify::changes
pub trait Modify {
    /// The [set](EnumSet) of fields that `Self` accesses on `I`.
    type Field: EnumSetType + fmt::Debug + Send + Sync;
    type Item;

    // TODO(perf): Change detection on context as well.
    /// An additional context **outside of `I`** that is relevant to operations on `I`.
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

/// A series of [`Prefab::Item`] that allow [`Prefab::Modify`] operations.
///
/// A `Prefab` may never exists. It's an interface to work with [`Modify`]
/// in a principled way through [`Resolver`]s.
pub trait Prefab {
    /// The individual element of the `Prefab`.
    type Item: Clone + fmt::Debug + Send + Sync;

    /// Operations allowed on [`Self::Item`].
    type Modify: Modify<Item = Self::Item> + fmt::Debug + Send + Sync;

    /// The underlying [`Self::Item`] storage.
    type Items: Indexed<Self> + Send + Sync;
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
