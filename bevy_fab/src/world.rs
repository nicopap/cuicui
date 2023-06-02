use bevy::ecs::prelude::Resource;

use fab::binding;

#[derive(Resource)]
pub struct PrefabWorld<M>(pub binding::World<M>);
impl<M> Default for PrefabWorld<M> {
    fn default() -> Self {
        PrefabWorld(Default::default())
    }
}
