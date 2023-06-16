//! Local entity-scopped data relevant to [`Modify`]s located in the bevy ECS.
use bevy::ecs::prelude::Component;

use fab::{binding, modify::Changing, resolve::Resolver};

use crate::{BevyModify, WorldBindings};

#[derive(Component)]
pub struct LocalBindings<M: BevyModify> {
    resolver: M::Resolver,
    pub root_data: Changing<M::Field, M::MakeItem>,
    pub bindings: binding::Local<M>,
}
impl<M: BevyModify> LocalBindings<M> {
    /// Update `to_update` with updated values from `world` and `self`-local bindings.
    ///
    /// Only the relevant sections of `to_update` are updated. The change trackers
    /// are then reset.
    pub fn update(
        &mut self,
        items: &mut M::Items<'_, '_, '_>,
        world: &WorldBindings<M>,
        ctx: &M::Context<'_>,
    ) {
        let Self { root_data, bindings, resolver } = self;

        // TODO(clean): this code should be in cuicui_fab
        let view = world.bindings.view_with_local(bindings).unwrap();
        resolver.update(items, root_data, view, ctx);
        root_data.reset_updated();
        bindings.reset_changes();
    }
    pub(crate) fn new(resolver: M::Resolver, root_data: M::MakeItem) -> Self {
        LocalBindings {
            resolver,
            root_data: Changing::new(root_data),
            bindings: Default::default(),
        }
    }
}
