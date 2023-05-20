use std::{any::Any, fmt};

use enumset::{EnumSet, EnumSetType};

pub trait Sequence<T: ?Sized> {
    fn get_mut(&mut self, index: usize) -> Option<&mut T>;
}

pub type FieldsOf<P> = EnumSet<<P as Prefab>::Field>;

pub trait Prefab {
    type Modifiers: Modify<Self> + fmt::Debug;
    type Field: EnumSetType;
    type Context;
    type Collection: Sequence<Self>;
    type FieldValue<'a>
    where
        Self: 'a;

    fn get_mut(&mut self, field: Self::Field) -> Self::FieldValue<'_>;
}
pub struct Tracked<P: Prefab> {
    pub(crate) updated: FieldsOf<P>,
    pub(crate) value: P,
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
