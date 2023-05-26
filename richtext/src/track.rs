//! Tracker structs to easily insert into ECS components you want to read
//! into rich text modifiers.

mod component;
mod pull;
mod reflect;
mod resource;

use bevy::{log::trace, prelude::World};

use crate::modifiers::TextModifiers;

pub(crate) type FetchBox =
    Box<dyn Fn(&mut Access, &World) -> Option<TextModifiers> + Send + Sync + 'static>;

fn some_content(input: impl std::fmt::Display) -> Option<TextModifiers> {
    let content = TextModifiers::content(input.to_string().into());
    trace!("Content of {content:?}");
    Some(content)
}

pub use component::{update_tracked_components, Tracked};
pub(crate) use pull::Target;
pub(crate) use reflect::make_tracker;
pub use resource::{update_tracked_resources, ResTrackers};

use self::pull::Access;

pub struct Tracker {
    // TODO(now): add a binding::Id field to query more efficiently the binding::World
    pub(crate) binding_name: String,
    // TODO(now): replace this with pull::Namespace,
    // pull::Namespace should store Range<u16> instead of &str, indexing binding_name
    pub(crate) fetch: FetchBox,
}
