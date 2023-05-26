use bevy::prelude::*;

use super::Tracker;
use crate::{plugin::WorldBindings, track::pull::Access};

pub fn update_tracked_resources(world: &mut World) {
    world.resource_scope(|world, mut world_bindings: Mut<WorldBindings>| {
        world.resource_scope(|world, mut trackers: Mut<ResTrackers>| {
            let ResTrackers { trackers, cache } = &mut *trackers;
            for Tracker { binding_name, fetch } in trackers.iter() {
                let Some(modify) = fetch(cache, world) else { continue; };
                trace!("Setting resource binding of {binding_name:?} to {modify:?}");
                world_bindings.set(binding_name, modify);
            }
        })
    })
}

/// Keeps track of resources that should be tracked.
///
/// Used in [`update_tracked_resources`] to update [`WorldBindings`] with the
/// content of tracked resources.
#[derive(Resource, Default)]
pub struct ResTrackers {
    trackers: Vec<Tracker>,
    cache: Access,
}
impl ResTrackers {
    pub fn extend(&mut self, iter: impl IntoIterator<Item = Tracker>) {
        self.trackers.extend(iter)
    }
}
