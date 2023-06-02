use bevy::ecs::prelude::Component;

use fab::{binding, prefab::Changing, prefab::Modify, resolve::Resolver};

use crate::PrefabWorld;

#[derive(Component)]
pub struct PrefabLocal<M: Modify, const R: usize> {
    resolver: Resolver<M, R>,
    pub root_data: Changing<M>,
    pub bindings: binding::Local<M>,
}
impl<M: Modify, const R: usize> PrefabLocal<M, R> {
    /// Update `to_update` with updated values from `world` and `self`-local bindings.
    ///
    /// Only the relevant sections of `to_update` are updated. The change trackers
    /// are then reset.
    pub fn update(
        &mut self,
        to_update: &mut M::Items,
        world: &PrefabWorld<M>,
        ctx: &M::Context<'_>,
    ) {
        let Self { root_data, bindings, resolver } = self;

        // TODO(clean): this code should be in cuicui_fab
        let view = world.0.view_with_local(bindings).unwrap();
        resolver.update(to_update, root_data, view, ctx);
        root_data.reset_updated();
        bindings.reset_changes();
    }
    pub(crate) fn new(resolver: Resolver<M, R>, root_data: M::Item) -> Self {
        PrefabLocal {
            resolver,
            root_data: Changing::new(root_data),
            bindings: Default::default(),
        }
    }
}
