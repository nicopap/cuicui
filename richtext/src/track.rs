//! Tracker structs to easily insert into ECS components you want to read
//! into rich text modifiers.

mod component;
mod resource;

use crate::ModifyBox;

type FetchBox = Box<dyn Fn(&bevy::prelude::World) -> Option<ModifyBox> + Send + Sync + 'static>;

fn some_content(input: impl std::fmt::Display) -> Option<ModifyBox> {
    Some(Box::new(crate::modifiers::Content::from(input)))
}

pub use component::{update_tracked_components, Tracked};
pub use resource::{update_tracked_resources, ResTrackers, ResourceTrackerExt};
