use std::{fmt, marker::PhantomData};

use bevy::{ecs::system::Command, prelude::*, reflect::Typed};

use super::{some_content, FetchBox, Tracker};
use crate::{plugin::WorldBindings, IntoModify};

pub fn update_tracked_resources(world: &mut World) {
    world.resource_scope(|world, mut world_bindings: Mut<WorldBindings>| {
        world.resource_scope(|world, trackers: Mut<ResTrackers>| {
            for Tracker { binding_name, fetch } in &trackers.0 {
                let Some(modify) = fetch(world) else { continue; };
                trace!("Setting resource binding of {binding_name:?} to {modify:?}");
                world_bindings.set(binding_name, modify);
            }
        })
    })
}

/// Keeps track of resources that should be tracked.
///
/// Used in [`update_tracked_resources`] to update [`WorldBindings`] with the
/// content of tracked resources.
#[derive(Resource, Default)]
pub struct ResTrackers(Vec<Tracker>);
impl ResTrackers {
    pub fn extend(&mut self, iter: impl IntoIterator<Item = Tracker>) {
        self.0.extend(iter)
    }
}

struct SetupResTracker<R: Resource> {
    tracker: Tracker,
    resource: R,
}
impl<R: Resource> Command for SetupResTracker<R> {
    fn write(self, world: &mut World) {
        let Self { tracker, resource } = self;
        let mut trackers = world.get_resource_or_insert_with(ResTrackers::default);
        trackers.0.push(tracker);
        world.insert_resource(resource);
    }
}
struct SetupInitResTracker<R: Resource + FromWorld> {
    tracker: Tracker,
    _r: PhantomData<R>,
}
impl<R: Typed + Resource + FromWorld> SetupInitResTracker<R> {
    fn new(fetch: FetchBox) -> Self {
        Self { tracker: Tracker::new::<R>(fetch), _r: PhantomData }
    }
}
impl<R: Resource + FromWorld> Command for SetupInitResTracker<R> {
    fn write(self, world: &mut World) {
        let mut trackers = world.get_resource_or_insert_with(ResTrackers::default);
        trackers.0.push(self.tracker);
        world.init_resource::<R>();
    }
}

/// [`App`] extension to add [`Resource`]s which value are kept in sync with
/// [`WorldBindings`].
pub trait ResourceTrackerExt {
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
    fn init_modify_resource<R: Typed + Resource + FromWorld + IntoModify + Clone>(
        &mut self,
    ) -> &mut Self {
        let fetch: FetchBox =
            Box::new(|world| Some(world.get_resource::<R>()?.clone().into_modify()));
        self.init_resource_with_fetch::<R>(fetch)
    }
    /// Inserts a [`Resource`] with provided value,
    /// and bind its [`IntoModify`] value to the [`get_short_name`] of its type.
    fn insert_modify_resource<R: Typed + Resource + IntoModify + Clone>(
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
impl ResourceTrackerExt for App {
    fn init_resource_with_fetch<R: Typed + Resource + FromWorld>(
        &mut self,
        fetch: FetchBox,
    ) -> &mut Self {
        SetupInitResTracker::<R>::new(fetch).write(&mut self.world);
        self
    }

    fn insert_resource_with_fetch<R: Typed + Resource>(
        &mut self,
        resource: R,
        fetch: FetchBox,
    ) -> &mut Self {
        let setup = SetupResTracker { tracker: Tracker::new::<R>(fetch), resource };
        setup.write(&mut self.world);
        self
    }
}
impl ResourceTrackerExt for Commands<'_, '_> {
    fn init_resource_with_fetch<R: Typed + Resource + FromWorld>(
        &mut self,
        fetch: FetchBox,
    ) -> &mut Self {
        self.add(SetupInitResTracker::<R>::new(fetch));
        self
    }

    fn insert_resource_with_fetch<R: Typed + Resource>(
        &mut self,
        resource: R,
        fetch: FetchBox,
    ) -> &mut Self {
        self.add(SetupResTracker { tracker: Tracker::new::<R>(fetch), resource });
        self
    }
}
