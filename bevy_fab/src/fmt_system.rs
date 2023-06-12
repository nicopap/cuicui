use bevy::{
    ecs::system::{ReadOnlySystemParam, SystemState},
    prelude::{FromWorld, World},
    reflect::Reflect,
};
use fab::binding::Entry;

pub trait IntoFmtSystem<M, TRGT: FmtSystem<M>> {
    fn into_fmt_system(self, world: &mut World) -> TRGT;
}
pub trait FmtSystem<M>: Send + Sync + 'static {
    fn run(&mut self, value: &dyn Reflect, entry: Entry<M>, world: &World);
}
pub struct ArbitraryFmt<F, S> {
    function: F,
    state: S,
}
impl<F, F0, M> FmtSystem<M> for ArbitraryFmt<F, SystemState<F0>>
where
    F0: ReadOnlySystemParam,
    F: FnMut(&dyn Reflect, Entry<M>, F0::Item<'_, '_>) + Send + Sync + 'static,
{
    fn run(&mut self, value: &dyn Reflect, entry: Entry<M>, world: &World) {
        let Self { function, state, .. } = self;
        (function)(value, entry, state.get(world));
    }
}
impl<F, M, F0> IntoFmtSystem<M, ArbitraryFmt<F, SystemState<F0>>> for F
where
    F0: ReadOnlySystemParam,
    F: FnMut(&dyn Reflect, Entry<M>, F0::Item<'_, '_>) + Send + Sync + 'static,
    for<'a> &'a mut F: FnMut(&dyn Reflect, Entry<M>, F0),
{
    fn into_fmt_system(self, world: &mut World) -> ArbitraryFmt<F, SystemState<F0>> {
        let state = SystemState::from_world(world);
        ArbitraryFmt { function: self, state }
    }
}
