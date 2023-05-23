use std::fmt;

#[cfg(doc)]
use crate::resolve::Resolver;

use enumset::{EnumSet, EnumSetType};

pub trait Indexed<T: ?Sized> {
    fn get_mut(&mut self, index: usize) -> Option<&mut T>;
}
impl<T> Indexed<T> for [T] {
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        <[T]>::get_mut(self, index)
    }
}
impl<T> Indexed<T> for Vec<T> {
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        <[T]>::get_mut(self, index)
    }
}

/// The [`Modify::Field`] of the [`Prefab::Modify`] of `P`.
pub type PrefabField<P> = Key<<P as Prefab>::Modify, <P as Prefab>::Item>;
type Key<M, I> = <M as Modify<I>>::Field;

/// Several [`Modify::Field`] of the [`Prefab::Modify`] of `P`.
pub type FieldsOf<P> = EnumSet<PrefabField<P>>;

/// The [`Modify::Context`] of the [`Prefab::Modify`] of `P`.
pub type PrefabContext<'a, P> = Ctx<'a, <P as Prefab>::Modify, <P as Prefab>::Item>;
type Ctx<'a, M, I> = <M as Modify<I>>::Context<'a>;

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
pub trait Modify<I: ?Sized> {
    /// The [set](EnumSet) elements of fields that `Self` accesses on `I`.
    type Field: EnumSetType;

    /// An additional context **outside of `I`** that is relevant to operations
    /// on `I`.
    type Context<'a>
    where
        Self: 'a;

    /// Apply this modifier to the `item`.
    ///
    /// It is important that `apply`:
    ///
    /// - only reads [`Self::Field`]s returned by [`Self::depends`].
    /// - only updates [`Self::Field`]s returned by [`Self::changes`].
    ///
    /// Otherwise, [`Resolver`] will fail to work properly.
    fn apply(&self, ctx: &Self::Context<'_>, item: &mut I) -> anyhow::Result<()>;

    /// On what data in `item` does this modifier depends?
    fn depends(&self) -> EnumSet<Self::Field>;

    /// What data does this `Modify` changes?
    fn changes(&self) -> EnumSet<Self::Field>;
}

/// A series of [`Prefab::Item`] that allow [`Prefab::Modify`] operations.
///
/// A `Prefab` may never exists. It's an interface to work with [`Modify`]
/// in a principled way through [`Resolver`]s.
pub trait Prefab {
    /// The individual element of the `Prefab`.
    type Item;

    /// Operations allowed on [`Self::Item`].
    type Modify: Modify<Self::Item> + fmt::Debug;

    /// The underlying [`Self::Item`] storage.
    type Items: Indexed<Self::Item>;
}

/// Holds a [`Prefab::Item`] and keeps track of changes to it.
///
/// You need to use [`Changing::update`] to access the `Item`, this will keep
/// track of the updated fields.
///
/// To reset the field update tracking, use [`Changing::reset_updated`].
pub struct Changing<P: Prefab> {
    pub(crate) updated: FieldsOf<P>,
    pub(crate) value: P::Item,
}
impl<P: Prefab> Changing<P> {
    /// Store this `value` in a `Changing`, with no updated field.
    pub fn new(value: P::Item) -> Self {
        Self { updated: EnumSet::EMPTY, value }
    }
    /// Update `self` with `f`, declaring that `update` is changed.
    ///
    /// If you change fields other than the ones in `updated`, they won't be
    /// tracked as changed. So make sure to properly declare which fields
    /// you are changing.
    pub fn update(&mut self, updated: FieldsOf<P>, f: impl FnOnce(&mut Self)) {
        self.updated |= updated;
        f(self);
    }
    /// Reset the change tracker state.
    pub fn reset_updated(&mut self) {
        self.updated = EnumSet::EMPTY;
    }
}
