use std::{any::Any, fmt};

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

pub type FieldsOf<P> = EnumSet<<P as Prefab>::Field>;

pub trait Prefab {
    type Modifiers: Modify<Self> + fmt::Debug;
    type Field: EnumSetType;
    type Context;
    type Collection: Indexed<Self>;
    type FieldValue<'a>
    where
        Self: 'a;

    fn get_mut(&mut self, field: Self::Field) -> Self::FieldValue<'_>;
}
pub struct Tracked<P: Prefab> {
    pub(crate) updated: FieldsOf<P>,
    pub(crate) value: P,
}
impl<P: Prefab> Tracked<P> {
    pub fn new(value: P) -> Self {
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

pub trait Modify<P: Prefab + ?Sized> {
    /// Apply this modifier to the `prefab`.
    fn apply(&self, ctx: &P::Context, prefab: &mut P) -> anyhow::Result<()>;

    /// On what data does this modifier depends?
    fn depends(&self) -> EnumSet<P::Field>;

    /// What data does this `Modify` changes?
    fn changes(&self) -> EnumSet<P::Field>;

    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn Modify<P>) -> bool;
    fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

/// A Boxed [`Modify`] trait object, with all necessary bounds to make it work
/// with bevy's [`Resource`] and [`Component`] types.
///
/// [`Resource`]: bevy::prelude::Resource
/// [`Component`]: bevy::prelude::Component
pub type ModifyBox<P> = Box<dyn Modify<P> + Send + Sync + 'static>;

impl<P: Prefab> PartialEq for dyn Modify<P> {
    fn eq(&self, other: &Self) -> bool {
        self.eq_dyn(other)
    }
}
impl<P: Prefab> PartialEq for ModifyBox<P> {
    fn eq(&self, other: &Self) -> bool {
        self.eq_dyn(&**other)
    }
}
impl<P: Prefab> fmt::Debug for dyn Modify<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_dyn(f)
    }
}
impl<P: Prefab> fmt::Debug for ModifyBox<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_dyn(f)
    }
}
