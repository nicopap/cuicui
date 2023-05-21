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
pub type Keys<T> = EnumSet<<T as PrefabSection>::Key>;

/// A prefab is built of several sections, each section may have several
/// components (`Key`).
pub trait PrefabSection {
    type Key: EnumSetType;
    type Value;
    fn get_mut(&mut self, field: Self::Key) -> &mut Self::Value;
}

pub trait Prefab {
    type Section: PrefabSection<Key = Self::Field>;
    type Modifiers: Modify<Self> + fmt::Debug;
    type Field: EnumSetType;
    type Context<'a>
    where
        Self: 'a;
    type Sections: Indexed<Self::Section>;
}
// TODO(clean): Rename this to ChangeTracked or smth
pub struct Tracked<T: PrefabSection> {
    pub(crate) updated: Keys<T>,
    pub(crate) value: T,
}
impl<T: PrefabSection> Tracked<T> {
    pub fn new(value: T) -> Self {
        Self { updated: EnumSet::EMPTY, value }
    }
    /// Update `self` with `f`, declaring that `update` is changed.
    ///
    /// If you change fields other than the ones in `updated`, they won't be
    /// tracked as changed. So make sure to properly declare which fields
    /// you are changing.
    pub fn update(&mut self, updated: Keys<T>, f: impl FnOnce(&mut Self)) {
        self.updated |= updated;
        f(self);
    }
    /// Reset the change tracker state.
    pub fn reset_updated(&mut self) {
        self.updated = EnumSet::EMPTY;
    }
}

/// A [`TextSection`] modifier.
///
/// A rich text [`Section`] may have an arbitary number of `Modify`s, modifying
/// the styling and content of a given section.
///
/// # Implementing `Modify`
///
/// You can create your own modifiers, the `as_any`, `eq_dyn` and
/// `debug_dyn` cannot be implemented at the trait level due to rust's trait object
/// rules, but they should all look similar.
///
/// The `apply` method is what should be interesting for you.
///
/// [`Section`]: crate::Section
pub trait Modify<P: Prefab + ?Sized> {
    /// Apply this modifier to the `prefab`.
    fn apply(&self, ctx: &P::Context<'_>, prefab: &mut P::Section) -> anyhow::Result<()>;

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
