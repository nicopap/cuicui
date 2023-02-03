use bevy::{
    ecs::system::{SystemParam, SystemParamItem},
    prelude::Entity,
};

/// Something spawned in the world which value can be read.
pub trait ReadWorldValue {
    type Value;
    type ReadParam<'w, 's>: SystemParam;
    fn read<'w, 's>(
        entity: Entity,
        params: &SystemParamItem<Self::ReadParam<'w, 's>>,
    ) -> Option<Self::Value>;
}
