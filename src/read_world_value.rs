use bevy::{
    ecs::system::{SystemParam, SystemParamItem},
    prelude::Entity,
};

/// Something spawned in the world which value can be read.
pub trait WorldValue {
    type Value;
    type ReadParam<'w, 's>: SystemParam;
    fn read(
        entity: Entity,
        params: &SystemParamItem<Self::ReadParam<'_, '_>>,
    ) -> Option<Self::Value>;
}
