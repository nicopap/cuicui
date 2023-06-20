use bevy::ecs::query::WorldQuery;
use bevy::prelude::{Component, World};

use crate::Builder;

pub trait Attribute {
    type Component: Component;
    type WorldQuery: WorldQuery;

    const READ: bool;
    const WRITE: bool;

    fn from_path(path: &'static str) -> Self;

    fn path(&self) -> &'static str;

    fn register(&self, world: &World, builder: &mut Builder) -> usize {
        let path = self.path();
        // TODO(bug): register component if not yet registered and this panics.
        let cid = world.component_id::<Self::Component>().unwrap();
        let aix = builder.a_map.len();
        *builder.a_map.entry((cid, path)).or_insert(aix)
    }
}
pub trait PathlessAttribute {
    type WithPath: Attribute;
}

#[rustfmt::skip]
mod attrs {
    use bevy::prelude::{Component, Deref, DerefMut};
    use std::marker::PhantomData;

    use super::{PathlessAttribute, Attribute};

    #[derive(Deref, DerefMut)]
    pub struct State<C>(C);
    pub struct Read<C>(PhantomData<C>);
    pub struct ReadWrite<C>(PhantomData<C>);
    pub struct WriteOnly<C>(PhantomData<C>);

    pub struct PState<C>(C, &'static str);
    pub struct PRead<C>(&'static str, PhantomData<C>);
    pub struct PReadWrite<C>(&'static str, PhantomData<C>);
    pub struct PWriteOnly<C>(&'static str, PhantomData<C>);

    impl<C: Component> PathlessAttribute for Read<C>      { type WithPath = PRead<C>; }
    impl<C: Component> PathlessAttribute for ReadWrite<C> { type WithPath = PReadWrite<C>; }
    impl<C: Component> PathlessAttribute for WriteOnly<C> { type WithPath = PWriteOnly<C>; }

    impl<C: Component> Attribute for PRead<C> {
        type Component = C;
        type WorldQuery = &'static C;

        const READ: bool = true;
        const WRITE: bool = false;

        fn from_path(path: &'static str) -> Self { Self(path, PhantomData) }
        fn path(&self) -> &'static str { self.0 }
    }
    impl<C: Component> Attribute for PReadWrite<C> {
        type Component = C;
        type WorldQuery = &'static mut C;

        const READ: bool = true;
        const WRITE: bool = true;

        fn from_path(path: &'static str) -> Self { Self(path, PhantomData) }
        fn path(&self) -> &'static str { self.0 }
    }
    impl<C: Component> Attribute for PWriteOnly<C> {
        type Component = C;
        type WorldQuery = &'static mut C;

        const READ: bool = false;
        const WRITE: bool = true;

        fn from_path(path: &'static str) -> Self { Self(path, PhantomData) }
        fn path(&self) -> &'static str { self.0 }
    }
}
pub use attrs::*;
