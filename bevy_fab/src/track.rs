//! Tracker structs to easily insert into ECS components you want to read
//! into modifiers.

mod component;
mod read;
mod write;

pub use component::{update_component_trackers_system, TrackerBundle};
pub use read::Read;
pub use write::Write;
