use std::fmt;

use bevy::{prelude::*, reflect::Typed, utils::get_short_name};

use super::{some_content, FetchBox};
use crate::{plugin::WorldBindings, IntoModify};

pub fn update_tracked_resources(world: &mut World) {
    world.resource_scope(|world, mut world_bindings: Mut<WorldBindings>| {
        world.resource_scope(|world, trackers: Mut<ResTrackers>| {
            for Tracker { binding_name, fetch } in &trackers.0 {
                let Some(modify) = fetch(world) else { continue; };
                world_bindings.set(binding_name, modify);
            }
        })
    })
}

pub(super) struct Tracker {
    pub(super) binding_name: &'static str,
    pub(super) fetch: FetchBox,
}
#[derive(Resource, Default)]
pub struct ResTrackers(pub(super) Vec<Tracker>);

/// [`App`] extension to add [`Resource`]s which value are kept in sync with
/// [`WorldBindings`].
pub trait AppResourceTrackerExt {
    /// Initialize a [`Resource`] with standard starting value,
    /// and keep track of its value in a rich text content binding.
    ///
    /// The binding name is the [`get_short_name`] of the resource type.
    fn init_tracked_resource<R: Typed + Resource + FromWorld + fmt::Display>(
        &mut self,
    ) -> &mut Self {
        let fetch: FetchBox = Box::new(|world| some_content(world.get_resource::<R>()?));
        self.init_resource_with_fetch::<R>(fetch)
    }
    /// Inserts a [`Resource`] with provided value,
    /// and keep track of its value in a rich text content binding.
    ///
    /// The binding name is the [`get_short_name`] of the resource type.
    fn insert_tracked_resource<R: Typed + Resource + fmt::Display>(
        &mut self,
        resource: R,
    ) -> &mut Self {
        let fetch: FetchBox = Box::new(|world| some_content(world.get_resource::<R>()?));
        self.insert_resource_with_fetch(resource, fetch)
    }
    /// Initialize a [`Resource`] with standard starting value,
    /// and keep track of its value as debug format in a rich text content binding.
    ///
    /// The binding name is the [`get_short_name`] of the resource type.
    fn init_debug_tracked_resource<R: Typed + Resource + FromWorld + fmt::Debug>(
        &mut self,
    ) -> &mut Self {
        let fetch: FetchBox =
            Box::new(|world| some_content(format!("{:?}", world.get_resource::<R>()?)));
        self.init_resource_with_fetch::<R>(fetch)
    }
    /// Inserts a [`Resource`] with provided value,
    /// and keep track of its value as debug format in a rich text content binding.
    ///
    /// The binding name is the [`get_short_name`] of the resource type.
    fn insert_debug_tracked_resource<R: Typed + Resource + fmt::Debug>(
        &mut self,
        resource: R,
    ) -> &mut Self {
        let fetch: FetchBox =
            Box::new(|world| some_content(format!("{:?}", world.get_resource::<R>()?)));
        self.insert_resource_with_fetch(resource, fetch)
    }
    /// Initialize a [`Resource`] with standard starting value,
    /// and bind its [`IntoModify`] value to the [`get_short_name`] of its type.
    fn init_tracked_modifier<R: Typed + Resource + FromWorld + IntoModify + Clone>(
        &mut self,
    ) -> &mut Self {
        let fetch: FetchBox =
            Box::new(|world| Some(world.get_resource::<R>()?.clone().into_modify()));
        self.init_resource_with_fetch::<R>(fetch)
    }
    /// Inserts a [`Resource`] with provided value,
    /// and bind its [`IntoModify`] value to the [`get_short_name`] of its type.
    fn insert_tracked_modifier<R: Typed + Resource + IntoModify + Clone>(
        &mut self,
        resource: R,
    ) -> &mut Self {
        let fetch: FetchBox =
            Box::new(|world| Some(world.get_resource::<R>()?.clone().into_modify()));
        self.insert_resource_with_fetch(resource, fetch)
    }
    fn init_resource_with_fetch<R: Typed + Resource + FromWorld>(
        &mut self,
        fetch: FetchBox,
    ) -> &mut Self;
    fn insert_resource_with_fetch<R: Typed + Resource>(
        &mut self,
        resource: R,
        fetch: FetchBox,
    ) -> &mut Self;
}
impl AppResourceTrackerExt for App {
    fn init_resource_with_fetch<R: Typed + Resource + FromWorld>(
        &mut self,
        fetch: FetchBox,
    ) -> &mut Self {
        let mut trackers = self.world.get_resource_or_insert_with(ResTrackers::default);
        let name = get_short_name(<R as Typed>::type_info().type_name());
        trackers.0.push(Tracker {
            // TODO(perf): hue, probably need to store a String or smth
            binding_name: Box::leak(name.into_boxed_str()),
            fetch,
        });
        self.init_resource::<R>()
    }

    fn insert_resource_with_fetch<R: Typed + Resource>(
        &mut self,
        resource: R,
        fetch: FetchBox,
    ) -> &mut Self {
        let mut trackers = self.world.get_resource_or_insert_with(ResTrackers::default);
        let name = get_short_name(<R as Typed>::type_info().type_name());
        trackers.0.push(Tracker {
            // TODO(perf): hue, probably need to store a String or smth
            binding_name: Box::leak(name.into_boxed_str()),
            fetch,
        });
        self.insert_resource(resource)
    }
}
