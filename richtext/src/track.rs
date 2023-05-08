//! Tracker structs to easily insert into ECS components you want to read
//! into rich text modifiers.

mod component;
mod pull;
mod reflect;
mod resource;

use bevy::{log::trace, prelude::World, reflect::Typed, utils::get_short_name};

use crate::ModifyBox;

pub(crate) type FetchBox = Box<dyn Fn(&World) -> Option<ModifyBox> + Send + Sync + 'static>;

fn some_content(input: impl std::fmt::Display) -> Option<ModifyBox> {
    let content = crate::modifiers::Content::from(input);
    trace!("Content of {content:?}");
    Some(Box::new(content))
}

pub use component::{update_tracked_components, Tracked};
pub(crate) use pull::Target;
pub(crate) use reflect::make_tracker;
pub use resource::{update_tracked_resources, ResTrackers, ResourceTrackerExt};

pub struct Tracker {
    pub(crate) binding_name: String,
    pub(crate) fetch: FetchBox,
}
impl Tracker {
    pub(crate) fn new<R: Typed>(fetch: FetchBox) -> Self {
        let binding_name = get_short_name(<R as Typed>::type_info().type_name());
        Self { binding_name, fetch }
    }
}
