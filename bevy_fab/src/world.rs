use bevy::ecs::prelude::Resource;

use fab::binding;

#[derive(Resource)]
pub struct WorldBindings<M>(pub binding::World<M>);
impl<M> Default for WorldBindings<M> {
    fn default() -> Self {
        WorldBindings(Default::default())
    }
}
