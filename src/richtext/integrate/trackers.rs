//! Tracker bundles to easily insert into ECS components you want to read

use std::fmt;

use bevy::{ecs::world::EntityRef, prelude::*, reflect::Typed, utils::get_short_name};

use super::{
    fetchers::{DynamicFetcher, FetchBox},
    GlobalRichTextBindings, IntoModify,
};
use crate::richtext::{Content, ModifyBox};

// TODO(clean): A safer alternative is to have `Tracker` bundles actually be
// bundles and expose a `new` constructor. I could then provide a macro
// if `Tracker::new(...)` proves to be too burdensome for users.
macro_rules! impl_hacky_bundle {
    (<$gen:ident : Component $(+ $($bound:ident)::+)+> $name:ident {
        $erase:expr
    }) => {
        /// # Safety
        ///
        /// Just copy/pasted from the macro impl in bevy, so supposedly this is safe,
        /// I don't do anything particularly suspicious (NOTE: actually wrong, the
        /// conversion between &'static str and Tracker would fall into  "suspicious" territory)
        unsafe impl<$gen: Component $(+ $($bound)::+)+> Bundle for $name<$gen> {
            fn component_ids(
                components: &mut bevy::ecs::component::Components,
                storages: &mut bevy::ecs::storage::Storages,
                ids: &mut impl FnMut(bevy::ecs::component::ComponentId),
            ) {
                Tracker::component_ids(components, storages, &mut *ids);
                $gen::component_ids(components, storages, &mut *ids);
            }

            unsafe fn from_components<C, F>(ctx: &mut C, func: &mut F) -> Self
            where
                // Ensure that the `OwningPtr` is used correctly
                F: for<'a> FnMut(&'a mut C) -> bevy::ptr::OwningPtr<'a>,
                Self: Sized,
            {
                Self {
                    0: Tracker::from_components(ctx, &mut *func).binding_name,
                    1: $gen::from_components(ctx, &mut *func),
                }
            }

            fn get_components(
                self,
                func: &mut impl FnMut(bevy::ecs::component::StorageType, bevy::ptr::OwningPtr<'_>),
            ) {
                let tracker = Tracker::new(self.0, $erase);
                tracker.get_components(&mut *func);
                self.1.get_components(&mut *func);
            }
        }
    };
}
macro_rules! track {
    (@build($ctor:expr, $binding_name:expr, $component:expr)) => {
        ($ctor)($binding_name, $component)
    };
    (d, $binding_name:ident , $component:expr)  => {
        track!(@build(DebugTracked::new, stringify!($binding_name), $component))
    };
    (d, $variable_name:ident)  => {
        track!(@build(DebugTracked::new, stringify!($variable_name), $variable_name))
    };
    (m, $binding_name:ident , $component:expr)  => {
        track!(@build(TrackedModifier::new, stringify!($binding_name), $component))
    };
    (m, $variable_name:ident)  => {
        track!(@build(TrackedModifier::new, stringify!($variable_name), $variable_name))
    };
    ($binding_name:ident , $component:expr) => {
        track!(@build(Tracked::new, stringify!($binding_name), $component))
    };
    ($variable_name:ident)  => {
        track!(@build(Tracked::new, stringify!($variable_name), $variable_name))
    };
}
// track!(m, binding_name, UiSize(foobar))
// track!(m, ui_size)

type ProtoFetch = fn(EntityRef) -> Option<ModifyBox>;

#[derive(Component)]
pub struct Tracker {
    binding_name: &'static str,
    proto_fetch: ProtoFetch,
}
impl Tracker {
    fn new(binding_name: &'static str, proto_fetch: ProtoFetch) -> Self {
        Self { binding_name, proto_fetch }
    }
}

// TODO(feat): a `Tracked` that accepts a Reflect + Path, as it allows tracking
// stuff like `Transform` positions.
pub struct Tracked<T: Component + fmt::Display>(pub &'static str, pub T);
pub struct DebugTracked<T: Component + fmt::Debug>(pub &'static str, pub T);
pub struct TrackedModifier<T: Component + IntoModify + Clone>(pub &'static str, pub T);
impl_hacky_bundle! {<T: Component + fmt::Display> Tracked {
    |entity| Some(Box::new(Content::from(entity.get::<T>()?)))
}}
impl_hacky_bundle! {<T: Component + fmt::Debug> DebugTracked {
    |entity| Some(Box::new(Content::from(format!("{:?}", entity.get::<T>()?))))
}}
impl_hacky_bundle! {<T: Component + IntoModify + Clone> TrackedModifier {
    |entity| Some(entity.get::<T>()?.clone().into_modify())
}}

pub fn update_tracked(world: &mut World, mut entities: Local<Vec<Entity>>) {
    world.resource_scope(|world, mut global_context: Mut<GlobalRichTextBindings>| {
        entities.extend(world.query_filtered::<Entity, With<Tracker>>().iter(world));
        for entity in entities.drain(..) {
            // TODO(perf): the 2 next Option<> can safely be unchecked_unwraped
            let Some(entity) = world.get_entity(entity) else { continue; };
            let Some(tracker) = entity.get::<Tracker>() else { continue; };
            let Some(modify) = (tracker.proto_fetch)(entity) else { continue; };
            global_context.bindings.insert(tracker.binding_name, modify);
        }
    })
}
pub fn update_resource_tracked(world: &mut World) {
    world.resource_scope(|world, mut global_context: Mut<GlobalRichTextBindings>| {
        world.resource_scope(|world, trackers: Mut<ResTrackers>| {
            for ResourceTracker { binding_name, fetch } in &trackers.0 {
                let Some(modify) = fetch.fetch(world) else { continue; };
                global_context.bindings.insert(binding_name, modify);
            }
        })
    })
}

struct ResourceTracker {
    binding_name: &'static str,
    fetch: FetchBox,
}
#[derive(Resource, Default)]
pub struct ResTrackers(Vec<ResourceTracker>);

pub trait AppResourceTrackerExt {
    /// Initialize a [`Resource`] with standard starting value,
    /// and keep track of its value in a rich text content binding.
    ///
    /// The binding name is the [`get_short_name`] of the resource type.
    fn init_tracked_resource<R: Typed + Resource + FromWorld + fmt::Display>(
        &mut self,
    ) -> &mut Self {
        let fetch =
            DynamicFetcher::new(|world| Some(Box::new(Content::from(world.get_resource::<R>()?))));
        self.init_resource_with_fetch::<R>(Box::new(fetch))
    }
    /// Inserts a [`Resource`] with provided value,
    /// and keep track of its value in a rich text content binding.
    ///
    /// The binding name is the [`get_short_name`] of the resource type.
    fn insert_tracked_resource<R: Typed + Resource + fmt::Display>(
        &mut self,
        resource: R,
    ) -> &mut Self {
        let fetch =
            DynamicFetcher::new(|world| Some(Box::new(Content::from(world.get_resource::<R>()?))));
        self.insert_resource_with_fetch(resource, Box::new(fetch))
    }
    /// Initialize a [`Resource`] with standard starting value,
    /// and keep track of its value as debug format in a rich text content binding.
    ///
    /// The binding name is the [`get_short_name`] of the resource type.
    fn init_debug_tracked_resource<R: Typed + Resource + FromWorld + fmt::Debug>(
        &mut self,
    ) -> &mut Self {
        let fetch = DynamicFetcher::new(|world| {
            Some(Box::new(Content::from(format!(
                "{:?}",
                world.get_resource::<R>()?
            ))))
        });
        self.init_resource_with_fetch::<R>(Box::new(fetch))
    }
    /// Inserts a [`Resource`] with provided value,
    /// and keep track of its value as debug format in a rich text content binding.
    ///
    /// The binding name is the [`get_short_name`] of the resource type.
    fn insert_debug_tracked_resource<R: Typed + Resource + fmt::Debug>(
        &mut self,
        resource: R,
    ) -> &mut Self {
        let fetch = DynamicFetcher::new(|world| {
            Some(Box::new(Content::from(format!(
                "{:?}",
                world.get_resource::<R>()?
            ))))
        });
        self.insert_resource_with_fetch(resource, Box::new(fetch))
    }
    /// Initialize a [`Resource`] with standard starting value,
    /// and bind its [`IntoModify`] value to the [`get_short_name`] of its type.
    fn init_tracked_modifier<R: Typed + Resource + FromWorld + IntoModify + Clone>(
        &mut self,
    ) -> &mut Self {
        let fetch =
            DynamicFetcher::new(|world| Some(world.get_resource::<R>()?.clone().into_modify()));
        self.init_resource_with_fetch::<R>(Box::new(fetch))
    }
    /// Inserts a [`Resource`] with provided value,
    /// and bind its [`IntoModify`] value to the [`get_short_name`] of its type.
    fn insert_tracked_modifier<R: Typed + Resource + IntoModify + Clone>(
        &mut self,
        resource: R,
    ) -> &mut Self {
        let fetch =
            DynamicFetcher::new(|world| Some(world.get_resource::<R>()?.clone().into_modify()));
        self.insert_resource_with_fetch(resource, Box::new(fetch))
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
        trackers.0.push(ResourceTracker {
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
        trackers.0.push(ResourceTracker {
            // TODO(perf): hue, probably need to store a String or smth
            binding_name: Box::leak(name.into_boxed_str()),
            fetch,
        });
        self.insert_resource(resource)
    }
}
