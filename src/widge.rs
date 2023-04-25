// pub mod action_button;
// pub mod checkbox;
pub mod composed;
pub mod event_button;
pub mod labelled;
pub mod list;
pub mod visual;

use bevy::{
    ecs::system::{EntityCommands, SystemParam, SystemParamItem},
    prelude::{DespawnRecursiveExt, Entity, In},
};

/// A value that has a `cuicui` representation.
///
/// It supports spawning a `Prefab`
pub trait Widge {
    fn spawn(&self, commands: EntityCommands);
    fn despawn(&self, commands: EntityCommands) {
        commands.despawn_recursive()
    }

    type ReadSystemParam<'w, 's>: SystemParam;
    fn read_from_ecs(
        entity: In<Entity>,
        params: &SystemParamItem<Self::ReadSystemParam<'_, '_>>,
    ) -> Option<Self>
    where
        Self: Sized;
}
