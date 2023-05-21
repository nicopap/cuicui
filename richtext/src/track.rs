//! Tracker structs to easily insert into ECS components you want to read
//! into rich text modifiers.

mod component;
mod pull;
mod reflect;
mod resource;

use bevy::{log::trace, prelude::World};

use crate::modifiers::ModifyBox;

pub(crate) type FetchBox = Box<dyn Fn(&World) -> Option<ModifyBox> + Send + Sync + 'static>;

fn some_content(input: impl std::fmt::Display) -> Option<ModifyBox> {
    let content = crate::modifiers::Content::from(input);
    trace!("Content of {content:?}");
    Some(Box::new(content))
}

pub use component::{update_tracked_components, Tracked};
pub(crate) use pull::Target;
pub(crate) use reflect::make_tracker;
pub use resource::{update_tracked_resources, ResTrackers};

pub struct Tracker {
    pub(crate) binding_name: String,
    pub(crate) fetch: FetchBox,
}
