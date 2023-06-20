use bevy::ecs::query::{QueryItem, WorldQuery};
use bevy::prelude::{Component, Entity, FromWorld, Mut, Query, Reflect, World};

use crate::access::{Access, Item, ParsedPaths, Paths};
use crate::access_registry::FnAccessRecorder;
use crate::{Bitset, Builder, ModifierFnMaker};

pub struct State<'a, T>(pub Mut<'a, T>);

pub trait ModifierParam {
    type ParamState;
    type QueryItem: WorldQuery;
    type InternalQueryItem: WorldQuery;
    type InitData;

    fn init(
        internal_world: &mut World,
        build: &mut FnAccessRecorder,
        data: Self::InitData,
    ) -> Self::ParamState;

    fn fetch<'a>(
        state: &mut Self::ParamState,
        world_item: QueryItem<'a, Self::QueryItem>,
        internal_item: Query<'_, '_, Self::InternalQueryItem>,
    ) -> Self;
}

impl<C: Component + Reflect, A: for<'a> Access<'a>> ModifierParam for Item<C, A> {
    type ParamState = ParsedPaths<A>;
    type QueryItem = &'static mut C;
    type InternalQueryItem = ();
    type InitData = Paths<A>;

    fn init(_: &mut World, stuff: &mut FnAccessRecorder, data: Self::InitData) -> Self::ParamState {
        Self::record(data, stuff);
        A::parse_path(data)
    }
    fn fetch<'a>(
        state: &mut Self::ParamState,
        world_item: QueryItem<'a, Self::QueryItem>,
        _: Query<'_, '_, Self::InternalQueryItem>,
    ) -> Self {
        Item::from_query(world_item, state)
    }
}
impl<'a, C: Component + FromWorld> ModifierParam for State<'a, C> {
    type ParamState = Entity;
    type QueryItem = ();
    type InternalQueryItem = &'static mut C;
    type InitData = Option<C>;

    fn init(
        internal_world: &mut World,
        _: &mut FnAccessRecorder,
        data: Self::InitData,
    ) -> Self::ParamState {
        let initial_state = data.unwrap_or_else(|| C::from_world(internal_world));
        internal_world.spawn(initial_state).id()
    }
    fn fetch<'s>(
        state: &mut Self::ParamState,
        _: QueryItem<'s, Self::QueryItem>,
        mut internal_query: Query<Self::InternalQueryItem>,
    ) -> Self {
        State(internal_query.get_mut(*state).unwrap())
    }
}

pub trait Modifier {
    fn state(&self) -> Option<Entity>;
    fn run(&mut self, world: &mut World);
}

pub struct ModifierState<F, S> {
    pub function: F,
    pub state: S,
}

pub trait IntoModifierState<T> {
    type Fun;
    type InitData;
    type State;

    fn into_modifier_state(
        self,
        w: &mut World,
        rec: &mut FnAccessRecorder,
        data: Self::InitData,
    ) -> ModifierState<Self::Fun, Self::State>;
}

impl<F, T0, T1> IntoModifierState<(T0, T1)> for F
where
    F: FnMut(T0, T1) + Send + Sync + 'static,
    T0: ModifierParam,
    T1: ModifierParam,
{
    type Fun = F;
    type InitData = (T0::InitData, T1::InitData);
    type State = (T0::ParamState, T1::ParamState);

    fn into_modifier_state(
        self,
        w: &mut World,
        rec: &mut FnAccessRecorder,
        (d0, d1): Self::InitData,
    ) -> ModifierState<Self::Fun, Self::State> {
        ModifierState {
            function: self,
            state: (T0::init(w, rec, d0), T1::init(w, rec, d1)),
        }
    }
}

impl<T0, T1, F> ModifierFnMaker<(T0, T1)> for ModifierState<(&'static str, &'static str), F>
where
    F: FnMut(T0, T1) + Send + Sync + 'static,
    for<'z> T0: Access<'z>,
    for<'z> T1: Access<'z>,
{
    fn register(&self, world: &bevy::prelude::World, builder: &mut Builder) -> (Bitset, Bitset) {
        let (mut depends, mut changes) = (Bitset::default(), Bitset::default());
        (depends, changes)
    }

    fn run(&mut self, world: &mut bevy::prelude::World) {
        todo!()
    }
}
// impl<T0, T1, T2, F> ModifierFnMaker<(T0, T1, T2)>
//     for ModifierPair<(&'static str, &'static str, &'static str), F>
// where
//     F: FnMut(T0, T1, T2) + Send + Sync + 'static,
//     T0: PathlessAttribute,
//     T1: PathlessAttribute,
//     T2: PathlessAttribute,
// {
//     fn register(&self, world: &bevy::prelude::World, builder: &mut Builder) -> (Bitset, Bitset) {
//         let (mut depends, mut changes) = (Bitset::default(), Bitset::default());
//         (depends, changes)
//     }

//     fn run(&mut self, world: &mut bevy::prelude::World) {
//         todo!()
//     }
// }
