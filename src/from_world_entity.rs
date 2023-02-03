use bevy::prelude::{Entity, World};

/// A thing assoicated with an entity that can be read from the world.
pub trait FromWorldEntity {
    fn read(entity: Entity, world: &World) -> Self;
}
