use bevy::{
    ecs::system::{SystemParam, SystemParamItem},
    prelude::Entity,
};

/// A thing assoicated with an entity that can be read from the world.
pub trait ExtractPrefab: Sized {
    type ExtractParam<'w, 's>: SystemParam;
    fn extract(
        entity: Entity,
        params: &SystemParamItem<Self::ExtractParam<'_, '_>>,
    ) -> Option<Self>;
}
