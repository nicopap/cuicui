//! Reflect [`TypeData`] to query efficiently individual components.
mod predefined;

use std::{iter, ops::Deref};

use bevy::{
    ecs::query::{QueryIter, QuerySingleError},
    prelude::{Component, DetectChanges, Entity, Mut, QueryState, Ref as BRef, With, World},
    reflect::{FromType, Reflect},
};

pub use predefined::BaseReflectQueryablePlugin;

pub type SingleResult<T> = Result<T, QuerySingleError>;

//
// versions of [`QueryState`] and [`QueryIter`] returning Reflect values,
// erased using trait objects.
//

pub trait TIter<'w, 's>: Iterator<Item = &'w dyn Reflect> {}

impl<'w, 's, C, F> TIter<'w, 's> for iter::Map<QueryIter<'w, 's, &'static C, ()>, F>
where
    C: Component + Reflect,
    F: Fn(&C) -> &dyn Reflect,
{
}

pub struct WIter<'a, 'w: 'a, 's: 'a>(pub Box<dyn TIter<'w, 's> + 'a>);

pub trait TState {
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WIter<'a, 'w, 's>;
}

impl<C: Component + Reflect> TState for QueryState<&'static C, ()> {
    /// Get an iterator of `&C`.
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WIter<'a, 'w, 's> {
        WIter(Box::new(self.iter(world).map(C::as_reflect)))
    }
}

//
// versions of [`QueryState`] and [`QueryIter`] returning `Ref`s values,
// erased using trait objects.
//

pub trait TrIter<'w, 's>: Iterator<Item = Ref<'w, dyn Reflect>> {}

impl<'w, 's, C, F> TrIter<'w, 's> for iter::Map<QueryIter<'w, 's, BRef<'static, C>, ()>, F>
where
    C: Component + Reflect,
    F: Fn(BRef<C>) -> Ref<dyn Reflect>,
{
}

pub struct WrIter<'a, 'w: 'a, 's: 'a>(pub Box<dyn TrIter<'w, 's> + 'a>);

pub trait TrState {
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WrIter<'a, 'w, 's>;
}

impl<C: Component + Reflect> TrState for QueryState<BRef<'static, C>, ()> {
    /// Get an iterator of `Ref<dyn Reflect>`.
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WrIter<'a, 'w, 's> {
        fn map_ref<C: Component + Reflect>(value: BRef<C>) -> Ref<dyn Reflect> {
            Ref::map_from(value, C::as_reflect)
        }
        WrIter(Box::new(self.iter(world).map(map_ref)))
    }
}

//
// versions of [`QueryState`] and [`QueryIter`] returning `Entity`ies.
// Erased using trait objects.
//

pub trait TeIter<'w, 's>: Iterator<Item = Entity> {}

impl<'w, 's, C: Component> TeIter<'w, 's> for QueryIter<'w, 's, Entity, With<C>> {}

pub struct WeIter<'a, 'w: 'a, 's: 'a>(pub Box<dyn TeIter<'w, 's> + 'a>);

pub trait TeState {
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WeIter<'a, 'w, 's>;
}

impl<C: Component> TeState for QueryState<Entity, With<C>> {
    /// Get an iterator of `Entity`.
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WeIter<'a, 'w, 's> {
        WeIter(Box::new(self.iter(world)))
    }
}

//
// versions of [`QueryState`] and [`QueryIter`] returning mutable values,
// erased using trait objects.
//

pub trait TmIter<'w, 's>: Iterator<Item = Mut<'w, dyn Reflect>> {}

impl<'w, 's, C, F> TmIter<'w, 's> for iter::Map<QueryIter<'w, 's, &'static mut C, ()>, F>
where
    C: Component + Reflect,
    F: Fn(Mut<C>) -> Mut<dyn Reflect>,
{
}

pub struct WmIter<'a, 'w: 'a, 's: 'a>(pub Box<dyn TmIter<'w, 's> + 'a>);

pub trait TmState {
    fn iter_mut<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w mut World) -> WmIter<'a, 'w, 's>;
}

impl<C: Component + Reflect> TmState for QueryState<&'static mut C, ()> {
    /// Get an iterator of `Mut<C>`.
    fn iter_mut<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w mut World) -> WmIter<'a, 'w, 's> {
        fn map_unchanged<C: Component + Reflect>(value: Mut<C>) -> Mut<dyn Reflect> {
            value.map_unchanged(C::as_reflect_mut)
        }
        WmIter(Box::new(self.iter_mut(world).map(map_unchanged::<C>)))
    }
}

//
// `ReflectQueryableFns`
//

pub struct ReflectQueryableIter(pub Box<dyn TState>);
pub struct ReflectQueryableIterEntities(pub Box<dyn TeState>);
pub struct ReflectQueryableIterMut(pub Box<dyn TmState>);
pub struct ReflectQueryableIterRef(pub Box<dyn TrState>);

#[derive(Clone)]
pub struct ReflectQueryableFns {
    pub get_single: fn(&mut World) -> SingleResult<&dyn Reflect>,
    pub get_single_entity: fn(&mut World) -> SingleResult<Entity>,
    pub get_single_ref: fn(&mut World) -> SingleResult<Ref<dyn Reflect>>,
    pub get_single_mut: fn(&mut World) -> SingleResult<Mut<dyn Reflect>>,

    pub iter: fn(&mut World) -> ReflectQueryableIter,
    pub iter_entities: fn(&mut World) -> ReflectQueryableIterEntities,
    pub iter_ref: fn(&mut World) -> ReflectQueryableIterRef,
    pub iter_mut: fn(&mut World) -> ReflectQueryableIterMut,
}

/// A [reflect trait] extending [`ReflectComponent`] with query methods.
///
/// [`ReflectComponent`] doesn't have methods to get
#[derive(Clone)]
pub struct ReflectQueryable(ReflectQueryableFns);

/// Get a single entity with the reflected queryable [`Component`].
impl ReflectQueryable {
    pub fn get_single<'a>(&self, world: &'a mut World) -> SingleResult<&'a dyn Reflect> {
        (self.0.get_single)(world)
    }
    pub fn get_single_ref<'a>(&self, world: &'a mut World) -> SingleResult<Ref<'a, dyn Reflect>> {
        (self.0.get_single_ref)(world)
    }
    pub fn get_single_mut<'a>(&self, world: &'a mut World) -> SingleResult<Mut<'a, dyn Reflect>> {
        (self.0.get_single_mut)(world)
    }
    pub fn get_single_entity<'a>(&self, world: &'a mut World) -> SingleResult<Entity> {
        (self.0.get_single_entity)(world)
    }
}

/// Iterate over all entities with the reflected queryable [`Component`].
impl ReflectQueryable {
    pub fn iter(&self, world: &mut World) -> ReflectQueryableIter {
        (self.0.iter)(world)
    }
    pub fn iter_entities(&self, world: &mut World) -> ReflectQueryableIterEntities {
        (self.0.iter_entities)(world)
    }
    pub fn iter_ref(&self, world: &mut World) -> ReflectQueryableIterRef {
        (self.0.iter_ref)(world)
    }
    pub fn iter_mut(&self, world: &mut World) -> ReflectQueryableIterMut {
        (self.0.iter_mut)(world)
    }
}

impl<C: Component + Reflect> FromType<C> for ReflectQueryable {
    fn from_type() -> Self {
        ReflectQueryable(ReflectQueryableFns {
            get_single: |world| {
                let component = world.query::<&C>().get_single(world)?;
                Ok(component.as_reflect())
            },
            get_single_ref: |world| {
                let value = world.query::<BRef<C>>().get_single(world)?;
                Ok(Ref::map_from(value, C::as_reflect))
            },
            get_single_mut: |world| {
                let query = world.query::<&mut C>().get_single_mut(world);
                Ok(query?.map_unchanged(C::as_reflect_mut))
            },
            get_single_entity: |world| world.query_filtered::<Entity, With<C>>().get_single(world),
            iter: |world| ReflectQueryableIter(Box::new(world.query::<&C>())),
            iter_mut: |world| ReflectQueryableIterMut(Box::new(world.query::<&mut C>())),
            iter_ref: |world| ReflectQueryableIterRef(Box::new(world.query::<BRef<C>>())),
            iter_entities: |world| {
                ReflectQueryableIterEntities(Box::new(world.query_filtered::<Entity, With<C>>()))
            },
        })
    }
}
#[derive(Debug)]
pub struct Ref<'a, T: ?Sized> {
    value: &'a T,
    is_added: bool,
    is_changed: bool,
    last_changed: u32,
}
impl<T: ?Sized> DetectChanges for Ref<'_, T> {
    fn is_added(&self) -> bool {
        self.is_added
    }
    fn is_changed(&self) -> bool {
        self.is_changed
    }
    fn last_changed(&self) -> u32 {
        self.last_changed
    }
}
impl<'w, T: ?Sized> Ref<'w, T> {
    pub fn map_from<U: ?Sized>(bevy: BRef<'w, U>, f: impl FnOnce(&U) -> &T) -> Self {
        Ref {
            is_added: bevy.is_added(),
            is_changed: bevy.is_changed(),
            last_changed: bevy.last_changed(),
            value: f(bevy.into_inner()),
        }
    }
}
impl<'w, T: ?Sized> From<BRef<'w, T>> for Ref<'w, T> {
    fn from(value: BRef<'w, T>) -> Self {
        Ref::map_from(value, |i| i)
    }
}
impl<'w, T: ?Sized> AsRef<T> for Ref<'w, T> {
    fn as_ref(&self) -> &T {
        self.value
    }
}
impl<'w, T: ?Sized> Deref for Ref<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}
impl<'w, 'a, T> IntoIterator for &'a Ref<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}
