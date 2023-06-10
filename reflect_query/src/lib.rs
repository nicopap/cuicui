//! Reflect [`TypeData`] to query efficiently individual components.
mod predefined;

use std::{iter, ops::Deref};

use bevy::{
    ecs::{query::QueryIter, query::QuerySingleError, world::EntityRef},
    prelude::{Component, DetectChanges, Entity, Mut, QueryState, Ref as BRef, With, World},
    reflect::{FromType, Reflect},
};

pub use predefined::BaseReflectQueryablePlugin;

pub type SingleResult<T> = Result<T, QuerySingleError>;

//
// versions of [`QueryState`] and [`QueryIter`] returning Reflect values,
// erased using trait objects.
//

trait TState {
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WIter<'a, 'w, 's>;
}
trait TIter<'w, 's>: Iterator<Item = &'w dyn Reflect> {}
pub struct WIter<'a, 'w: 'a, 's: 'a>(Box<dyn TIter<'w, 's> + 'a>);

impl<'w, 's, C, F> TIter<'w, 's> for iter::Map<QueryIter<'w, 's, &'static C, ()>, F>
where
    C: Component + Reflect,
    F: Fn(&C) -> &dyn Reflect,
{
}

impl<'a, 'w: 'a, 's: 'a> Iterator for WIter<'a, 'w, 's> {
    type Item = &'w dyn Reflect;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
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

trait TrState {
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WrIter<'a, 'w, 's>;
}
trait TrIter<'w, 's>: Iterator<Item = Ref<'w, dyn Reflect>> {}
pub struct WrIter<'a, 'w: 'a, 's: 'a>(Box<dyn TrIter<'w, 's> + 'a>);

impl<'w, 's, C, F> TrIter<'w, 's> for iter::Map<QueryIter<'w, 's, BRef<'static, C>, ()>, F>
where
    C: Component + Reflect,
    F: Fn(BRef<C>) -> Ref<dyn Reflect>,
{
}

impl<'a, 'w: 'a, 's: 'a> Iterator for WrIter<'a, 'w, 's> {
    type Item = Ref<'w, dyn Reflect>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
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

trait TeState {
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WeIter<'a, 'w, 's>;
}
trait TeIter<'w, 's>: Iterator<Item = Entity> {}
pub struct WeIter<'a, 'w: 'a, 's: 'a>(Box<dyn TeIter<'w, 's> + 'a>);

impl<'w, 's, C: Component> TeIter<'w, 's> for QueryIter<'w, 's, Entity, With<C>> {}

impl<'a, 'w: 'a, 's: 'a> Iterator for WeIter<'a, 'w, 's> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
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

trait TmState {
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w mut World) -> WmIter<'a, 'w, 's>;
}
trait TmIter<'w, 's>: Iterator<Item = Mut<'w, dyn Reflect>> {}
pub struct WmIter<'a, 'w: 'a, 's: 'a>(Box<dyn TmIter<'w, 's> + 'a>);

impl<'w, 's, C, F> TmIter<'w, 's> for iter::Map<QueryIter<'w, 's, &'static mut C, ()>, F>
where
    C: Component + Reflect,
    F: Fn(Mut<C>) -> Mut<dyn Reflect>,
{
}

impl<'a, 'w: 'a, 's: 'a> Iterator for WmIter<'a, 'w, 's> {
    type Item = Mut<'w, dyn Reflect>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<C: Component + Reflect> TmState for QueryState<&'static mut C, ()> {
    /// Get an iterator of `Mut<C>`.
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w mut World) -> WmIter<'a, 'w, 's> {
        fn map_unchanged<C: Component + Reflect>(value: Mut<C>) -> Mut<dyn Reflect> {
            value.map_unchanged(C::as_reflect_mut)
        }
        WmIter(Box::new(self.iter_mut(world).map(map_unchanged::<C>)))
    }
}

//
// `ReflectQueryableFns`
//

pub struct ReflectQueryableIter(Box<dyn TState>);
pub struct ReflectQueryableIterEntities(Box<dyn TeState>);
pub struct ReflectQueryableIterMut(Box<dyn TmState>);
pub struct ReflectQueryableIterRef(Box<dyn TrState>);

impl ReflectQueryableIter {
    pub fn iter<'a, 'w: 'a, 's: 'a>(
        &'s mut self,
        world: &'w World,
    ) -> impl Iterator<Item = &'w dyn Reflect> + 'a {
        self.0.iter(world).0
    }
}

impl ReflectQueryableIterEntities {
    pub fn iter<'a, 'w: 'a, 's: 'a>(
        &'s mut self,
        world: &'w World,
    ) -> impl Iterator<Item = Entity> + 'a {
        self.0.iter(world).0
    }
}

impl ReflectQueryableIterMut {
    pub fn iter_mut<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w mut World) -> WmIter<'a, 'w, 's> {
        self.0.iter(world)
    }
}

impl ReflectQueryableIterRef {
    pub fn iter<'a, 'w: 'a, 's: 'a>(
        &'s mut self,
        world: &'w World,
    ) -> impl Iterator<Item = Ref<'w, dyn Reflect>> + 'a {
        self.0.iter(world).0
    }
}

#[derive(Clone)]
pub struct ReflectQueryableFns {
    pub reflect_ref: fn(EntityRef) -> Option<Ref<dyn Reflect>>,

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

impl ReflectQueryable {
    pub fn get(&self) -> &ReflectQueryableFns {
        &self.0
    }
}

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
    pub fn get_single_entity(&self, world: &mut World) -> SingleResult<Entity> {
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
            reflect_ref: |entity| {
                let world = entity.world();
                let last_change_tick = world.last_change_tick();
                let change_tick = world.read_change_tick();
                let ticks = entity.get_change_ticks::<C>()?;

                let with_ticks = Ref {
                    value: entity.get::<C>()?,
                    is_added: ticks.is_added(last_change_tick, change_tick),
                    is_changed: ticks.is_changed(last_change_tick, change_tick),
                };
                Some(with_ticks.map(C::as_reflect))
            },
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
#[derive(Debug, Clone, Copy)]
pub struct Ref<'a, T: ?Sized> {
    value: &'a T,
    is_added: bool,
    is_changed: bool,
}
impl<'w, T: ?Sized> Ref<'w, T> {
    pub fn is_added(&self) -> bool {
        self.is_added
    }
    pub fn is_changed(&self) -> bool {
        self.is_changed
    }
    pub fn into_inner(self) -> &'w T {
        self.value
    }
    pub fn map_failable<E, U: ?Sized>(
        self,
        f: impl FnOnce(&T) -> Result<&U, E>,
    ) -> Result<Ref<'w, U>, E> {
        Ok(Ref {
            value: f(self.value)?,
            is_added: self.is_added,
            is_changed: self.is_changed,
        })
    }
    pub fn map<U: ?Sized>(self, f: impl FnOnce(&T) -> &U) -> Ref<'w, U> {
        Ref {
            value: f(self.value),
            is_added: self.is_added,
            is_changed: self.is_changed,
        }
    }
    pub fn map_from<U: ?Sized>(bevy: BRef<'w, U>, f: impl FnOnce(&U) -> &T) -> Self {
        Ref {
            is_added: bevy.is_added(),
            is_changed: bevy.is_changed(),
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
