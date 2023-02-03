pub mod containers;
pub mod from_world_entity;
pub mod layout;
pub mod prefab;
pub mod read_world_value;

use prefab::Prefab;
use read_world_value::ReadWorldValue;

/// A value that has a `cuicui` representation.
///
/// It supports spawning a `Prefab`
pub trait UiControl: Prefab + ReadWorldValue {}
