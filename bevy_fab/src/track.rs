//! Tracker structs to easily insert into ECS components you want to read
//! into rich text modifiers.

mod component;
mod hook;
mod read;
mod write;

pub use component::{update_component_trackers_system, TrackerBundle};
pub use hook::{update_hooked, Hook, Hooks};
pub use read::Read;
pub use write::Write;
