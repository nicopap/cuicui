use bevy::ecs::prelude::Resource;

use fab::binding;

use crate::BevyPrefab;

#[derive(Resource)]
pub struct PrefabWorld<P: BevyPrefab>(pub binding::World<P>);
impl<P: BevyPrefab> Default for PrefabWorld<P> {
    fn default() -> Self {
        PrefabWorld(Default::default())
    }
}
