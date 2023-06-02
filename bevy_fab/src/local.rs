use bevy::ecs::prelude::Component;

use fab::{binding, prefab::Changing, prefab::PrefabContext, resolve::Resolver};

use crate::{BevyPrefab, PrefabWorld};

#[derive(Component)]
pub struct PrefabLocal<P: BevyPrefab, const R: usize> {
    resolver: Resolver<P, R>,
    pub root_data: Changing<P::Modify>,
    pub bindings: binding::Local<P>,
}
impl<P: BevyPrefab, const R: usize> PrefabLocal<P, R> {
    /// Update `to_update` with updated values from `world` and `self`-local bindings.
    ///
    /// Only the relevant sections of `to_update` are updated. The change trackers
    /// are then reset.
    pub fn update(
        &mut self,
        to_update: &mut P::Items,
        world: &PrefabWorld<P>,
        ctx: &PrefabContext<P>,
    ) {
        let Self { root_data, bindings, resolver } = self;

        // TODO(clean): this code should be in cuicui_fab
        let view = world.0.view_with_local(bindings).unwrap();
        resolver.update(to_update, root_data, view, ctx);
        root_data.reset_updated();
        bindings.reset_changes();
    }
    pub(crate) fn new(resolver: Resolver<P, R>, root_data: P::Item) -> Self {
        PrefabLocal {
            resolver,
            root_data: Changing::new(root_data),
            bindings: Default::default(),
        }
    }
}
