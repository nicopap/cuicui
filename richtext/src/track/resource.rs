use bevy::prelude::*;

use super::Tracker;
use crate::plugin::WorldBindings;

pub fn update_tracked_resources(world: &mut World) {
    world.resource_scope(|world, mut world_bindings: Mut<WorldBindings>| {
        world.resource_scope(|world, trackers: Mut<ResTrackers>| {
            for Tracker { binding_name, fetch } in &trackers.0 {
                let Some(modify) = fetch(world) else { continue; };
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
pub struct ResTrackers(Vec<Tracker>);
impl ResTrackers {
    pub fn extend(&mut self, iter: impl IntoIterator<Item = Tracker>) {
        self.0.extend(iter)
    }
}
