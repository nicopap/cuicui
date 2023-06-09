//! Reflect [`TypeData`] to query efficiently individual components.
use std::{iter, ops::Deref};

use bevy::{
    ecs::query::{QueryIter, QuerySingleError},
    prelude::{Component, DetectChanges, Mut, QueryState, Ref as BRef, World},
    reflect::{FromType, Reflect},
};

pub type SingleResult<T> = Result<T, QuerySingleError>;

//
// versions of [`QueryState`] and [`QueryIter`] returning Reflect values,
// erased using trait objects.
//

pub struct WIter<'a, 'w: 'a, 's: 'a>(pub Box<dyn TIter<'w, 's> + 'a>);

pub trait TState {
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WIter<'a, 'w, 's>;
}

impl<C: Component + Reflect> TState for QueryState<&'static C, ()> {
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WIter<'a, 'w, 's> {
        WIter(Box::new(self.iter(world).map(C::as_reflect)))
    }
}

pub trait TIter<'w, 's>: Iterator<Item = &'w dyn Reflect> {}

impl<'w, 's, C, F> TIter<'w, 's> for iter::Map<QueryIter<'w, 's, &'static C, ()>, F>
where
    C: Component + Reflect,
    F: Fn(&C) -> &dyn Reflect,
{
}

//
// versions of [`QueryState`] and [`QueryIter`] returning `Ref`s values,
// erased using trait objects.
//

fn map_ref<C: Component + Reflect>(value: BRef<C>) -> Ref<dyn Reflect> {
    Ref::map_from(value, C::as_reflect)
}

pub struct WrIter<'a, 'w: 'a, 's: 'a>(pub Box<dyn TrIter<'w, 's> + 'a>);

pub trait TrState {
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WrIter<'a, 'w, 's>;
}

impl<C: Component + Reflect> TrState for QueryState<BRef<'static, C>, ()> {
    fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w World) -> WrIter<'a, 'w, 's> {
        WrIter(Box::new(self.iter(world).map(map_ref)))
    }
}

pub trait TrIter<'w, 's>: Iterator<Item = Ref<'w, dyn Reflect>> {}

impl<'w, 's, C, F> TrIter<'w, 's> for iter::Map<QueryIter<'w, 's, BRef<'static, C>, ()>, F>
where
    C: Component + Reflect,
    F: Fn(BRef<C>) -> Ref<dyn Reflect>,
{
}

//
// versions of [`QueryState`] and [`QueryIter`] returning mutable values,
// erased using trait objects.
//

fn map_unchanged<C: Component + Reflect>(value: Mut<C>) -> Mut<dyn Reflect> {
    value.map_unchanged(C::as_reflect_mut)
}

pub struct WmIter<'a, 'w: 'a, 's: 'a>(pub Box<dyn TmIter<'w, 's> + 'a>);

pub trait TmState {
    fn iter_mut<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w mut World) -> WmIter<'a, 'w, 's>;
}

impl<C: Component + Reflect> TmState for QueryState<&'static mut C, ()> {
    fn iter_mut<'a, 'w: 'a, 's: 'a>(&'s mut self, world: &'w mut World) -> WmIter<'a, 'w, 's> {
        WmIter(Box::new(self.iter_mut(world).map(map_unchanged::<C>)))
    }
}

pub trait TmIter<'w, 's>: Iterator<Item = Mut<'w, dyn Reflect>> {}

impl<'w, 's, C, F> TmIter<'w, 's> for iter::Map<QueryIter<'w, 's, &'static mut C, ()>, F>
where
    C: Component + Reflect,
    F: Fn(Mut<C>) -> Mut<dyn Reflect>,
{
}

//
// `ReflectQueryFns`
//

pub struct ReflectQueryIter(pub Box<dyn TState>);
pub struct ReflectQueryIterMut(pub Box<dyn TmState>);
pub struct ReflectQueryIterRef(pub Box<dyn TrState>);

#[derive(Clone)]
pub struct ReflectQueryFns {
    pub iter: fn(&mut World) -> ReflectQueryIter,
    pub iter_ref: fn(&mut World) -> ReflectQueryIterRef,
    pub iter_mut: fn(&mut World) -> ReflectQueryIterMut,
    pub get_single: fn(&mut World) -> SingleResult<&dyn Reflect>,
    pub get_single_ref: fn(&mut World) -> SingleResult<Ref<dyn Reflect>>,
    pub get_single_mut: fn(&mut World) -> SingleResult<Mut<dyn Reflect>>,
}
#[derive(Clone)]
pub struct ReflectQuery(ReflectQueryFns);
impl ReflectQuery {
    pub fn iter(&self, world: &mut World) -> ReflectQueryIter {
        (self.0.iter)(world)
    }
    pub fn iter_mut(&self, world: &mut World) -> ReflectQueryIterMut {
        (self.0.iter_mut)(world)
    }
    pub fn get_single<'a>(&self, world: &'a mut World) -> SingleResult<&'a dyn Reflect> {
        (self.0.get_single)(world)
    }
    pub fn get_single_ref<'a>(&self, world: &'a mut World) -> SingleResult<Ref<'a, dyn Reflect>> {
        (self.0.get_single_ref)(world)
    }
    pub fn get_single_mut<'a>(&self, world: &'a mut World) -> SingleResult<Mut<'a, dyn Reflect>> {
        (self.0.get_single_mut)(world)
    }
}
impl<C: Component + Reflect> FromType<C> for ReflectQuery {
    fn from_type() -> Self {
        ReflectQuery(ReflectQueryFns {
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
            iter: |world| ReflectQueryIter(Box::new(world.query::<&C>())),
            iter_mut: |world| ReflectQueryIterMut(Box::new(world.query::<&mut C>())),
            iter_ref: |world| ReflectQueryIterRef(Box::new(world.query::<BRef<C>>())),
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
