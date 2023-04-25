use std::any::{Any, TypeId};

use bevy::{
    ecs::{
        query::{ROQueryItem, WorldQuery},
        system::{EntityCommands, SystemParamItem},
    },
    prelude::*,
    utils::HashMap,
};

use crate::Widge;

#[derive(Default)]
pub struct Style(HashMap<TypeId, Box<dyn Any>>);
impl Style {
    pub fn with<T: 'static>(&mut self, value: T) {
        self.0.insert(TypeId::of::<T>(), Box::new(value));
    }
}

pub struct Compose<W: Widge, T: FromEcs + Clone + Bundle> {
    pub widge: W,
    pub bundle: T,
}
impl<W: Widge, T: FromEcs + Clone + Bundle> Compose<W, T> {
    pub fn with<U: Component + Default + Clone>(self, addon: U) -> Compose<W, (U, T)> {
        Compose { widge: self.widge, bundle: (addon, self.bundle) }
    }
    pub fn with_style(widge: W, map: &Style) -> Self {
        Compose { widge, bundle: T::from_map(&map.0) }
    }
}
pub trait WidgeComposeExt: Sized + Widge {
    fn compose<U: Component + Default + Clone>(self, addon: U) -> Compose<Self, (U, ())>;
}
impl<T: Widge> WidgeComposeExt for T {
    fn compose<U: Component + Default + Clone>(self, addon: U) -> Compose<Self, (U, ())> {
        Compose { widge: self, bundle: (addon, ()) }
    }
}
pub trait FromEcs: 'static {
    type QueryP: WorldQuery;
    fn from_query_item(item: ROQueryItem<'_, Self::QueryP>) -> Self;
    fn from_map(map: &HashMap<TypeId, Box<dyn Any>>) -> Self;
}
impl FromEcs for () {
    type QueryP = ();
    fn from_query_item(_: ROQueryItem<'_, Self::QueryP>) -> Self {}
    fn from_map(_: &HashMap<TypeId, Box<dyn Any>>) -> Self {}
}
impl<T: Component + Clone + Default, U: FromEcs> FromEcs for (T, U) {
    type QueryP = (&'static T, U::QueryP);

    fn from_query_item(item: ROQueryItem<'_, Self::QueryP>) -> Self {
        (item.0.clone(), U::from_query_item(item.1))
    }

    fn from_map(map: &HashMap<TypeId, Box<dyn Any>>) -> Self {
        (
            map.get(&TypeId::of::<T>())
                .map_or_else(T::default, |t| t.downcast_ref::<T>().unwrap().clone()),
            U::from_map(map),
        )
    }
}

impl<W: Widge, T: FromEcs + Clone + Bundle> Widge for Compose<W, T> {
    fn spawn(&self, mut commands: EntityCommands) {
        commands.insert(self.bundle.clone());
        self.widge.spawn(commands);
    }

    type ReadSystemParam<'w, 's> = (W::ReadSystemParam<'w, 's>, Query<'w, 's, T::QueryP>);

    fn read_from_ecs(
        entity: In<Entity>,
        (widge, bundle): &SystemParamItem<Self::ReadSystemParam<'_, '_>>,
    ) -> Option<Self>
    where
        Self: Sized,
    {
        Some(Compose {
            widge: W::read_from_ecs(In(entity.0), widge)?,
            bundle: T::from_query_item(bundle.get(entity.0).ok()?),
        })
    }
}
