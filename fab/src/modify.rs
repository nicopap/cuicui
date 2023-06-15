use std::fmt;

#[cfg(doc)]
use crate::resolve::DepsResolver;
use crate::resolve::Resolver;

use enumset::{EnumSet, EnumSetType};

pub trait MakeItem<'a, I: 'a> {
    fn make_item<'b, 'c>(&'b self, item: &'c mut I)
    where
        'a: 'c;
    fn as_item(&'a mut self) -> I;
}
impl<'a, T: Clone> MakeItem<'a, &'a mut T> for T {
    fn make_item<'b, 'c>(&'b self, item: &'c mut &'a mut T)
    where
        'a: 'c,
    {
        **item = self.clone();
    }
    fn as_item(&mut self) -> &mut T {
        self
    }
}
impl<'a, T1: Clone, T2: Clone> MakeItem<'a, (&'a mut T1, &'a mut T2)> for (T1, T2) {
    fn make_item<'b, 'c>(&self, item: &'c mut (&'a mut T1, &'a mut T2))
    where
        'a: 'c,
    {
        *item.0 = self.0.clone();
        *item.1 = self.1.clone();
    }
    fn as_item(&mut self) -> (&mut T1, &mut T2) {
        let (t1, t2) = self;
        (t1, t2)
    }
}

pub trait Indexed<M: Modify + ?Sized> {
    fn get_mut(&mut self, index: usize) -> Option<M::Item<'_>>;
}
impl<T, M: for<'a> Modify<Item<'a> = &'a mut T>> Indexed<M> for [T] {
    fn get_mut(&mut self, index: usize) -> Option<M::Item<'_>> {
        <[T]>::get_mut(self, index)
    }
}
impl<T, M: for<'a> Modify<Item<'a> = &'a mut T>> Indexed<M> for Vec<T> {
    fn get_mut(&mut self, index: usize) -> Option<M::Item<'_>> {
        <[T]>::get_mut(self, index)
    }
}

/// Several [`Modify::Field`]s.
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
/// [`impl_modify!`]: crate::impl_modify
pub trait Modify: Clone + fmt::Debug {
    type MakeItem: for<'a> MakeItem<'a, Self::Item<'a>> + Clone + fmt::Debug + Send + Sync;

    /// The type on which `Modify` operates
    type Item<'a>;

    /// The underlying [`Self::Item`] storage.
    type Items<'a, 'b, 'c>: Indexed<Self> + Send + Sync
    where
        Self: 'a;

    /// The [set](EnumSet) of fields that `Self` accesses on `Item`.
    type Field: EnumSetType + fmt::Debug + Send + Sync;

    // TODO(perf): Change detection on context as well.
    /// An additional context **outside of `Item`** that is relevant to operations on `Item`.
    type Context<'a>
    where
        Self: 'a;

    /// The [`Resolver`] used for this `Modify`.
    type Resolver: Resolver<Self> + fmt::Debug + Send + Sync;

    /// Apply this modifier to the [`Self::Item`].
    ///
    /// It is important that `apply`:
    ///
    /// - only reads [`Self::Field`]s returned by [`Self::depends`].
    /// - only updates [`Self::Field`]s returned by [`Self::changes`].
    ///
    /// Otherwise, [`Resolver`] will fail to work properly.
    fn apply(&self, ctx: &Self::Context<'_>, item: Self::Item<'_>) -> anyhow::Result<()>;

    /// On what data in [`Self::Item`] does this modifier depends?
    fn depends(&self) -> EnumSet<Self::Field>;

    /// What data in [`Self::Item`] does this `Modify` changes?
    fn changes(&self) -> EnumSet<Self::Field>;
}

/// Holds a [`Modify::Item`] and keeps track of changes to it.
///
/// You need to use [`Changing::update`] to access the `Item`, this will keep
/// track of the updated fields.
///
/// To reset the field update tracking, use [`Changing::reset_updated`].
pub struct Changing<F: EnumSetType, T> {
    pub(crate) updated: EnumSet<F>,
    pub(crate) value: T,
}
impl<F: EnumSetType, T> Changing<F, T> {
    /// Store this `value` in a `Changing`, with no updated field.
    pub fn new(value: T) -> Self {
        Self { updated: EnumSet::EMPTY, value }
    }
    /// Update `self` with `f`, declaring that `update` is changed.
    ///
    /// If you change fields other than the ones in `updated`, they won't be
    /// tracked as changed. So make sure to properly declare which fields
    /// you are changing.
    pub fn update(&mut self, updated: F, f: impl FnOnce(&mut T)) {
        self.updated |= updated;
        f(&mut self.value);
    }
    /// Reset the change tracker state.
    pub fn reset_updated(&mut self) {
        self.updated = EnumSet::EMPTY;
    }
}
