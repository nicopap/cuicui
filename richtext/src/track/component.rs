use std::fmt;

use bevy::{ecs::world::EntityRef, prelude::*};

use super::some_content;
use crate::{plugin::WorldBindings, IntoModify, ModifyBox};

/// Add a component and keep track of its value in [`WorldBindings`],
/// this is a soft wrapper around [`Tracked`] methods.
///
/// # Syntax
///
/// `track!` supports several call convention:
///
/// - `track!("<X>", <identifier>, <expression>)`
/// - `track!("<X>", <identifier>)`
/// - `track!(<identifier>, <expression>)`
/// - `track!(<identifier>)`
///
/// TODO(doc): complete informations.
#[macro_export]
macro_rules! track {
    (@build($ctor:expr, $binding_name:expr, $component:expr)) => {
        ($ctor)($binding_name, $component)
    };
    ("d", $binding_name:ident , $component:expr)  => {
        track!(@build($crate::Tracked::debug, stringify!($binding_name), $component))
    };
    ("d", $variable_name:ident)  => {
        track!(@build($crate::Tracked::debug, stringify!($variable_name), $variable_name))
    };
    ("m", $binding_name:ident , $component:expr)  => {
        track!(@build($crate::Tracked::modifier, stringify!($binding_name), $component))
    };
    ("m", $variable_name:ident)  => {
        track!(@build($crate::Tracked::modifier, stringify!($variable_name), $variable_name))
    };
    ($binding_name:ident , $component:expr) => {
        track!(@build($crate::Tracked::content, stringify!($binding_name), $component))
    };
    ($variable_name:ident)  => {
        track!(@build($crate::Tracked::content, stringify!($variable_name), $variable_name))
    };
}
// track!(m, binding_name, UiSize(foobar))
// track!(m, ui_size)

type ProtoFetch = fn(EntityRef) -> Option<ModifyBox>;

/// Track a component of this entity and keep the binding `binding_name`
/// in [`WorldBindings`] up to date with its value.
#[derive(Component)]
struct Tracker {
    binding_name: &'static str,
    proto_fetch: ProtoFetch,
}

/// Add a tracked `T` which value will be kept in sync in [`WorldBindings`].
///
/// Use one of `Tracked`'s method or the [`track!`] macro to create a tracked
/// component.
#[derive(Bundle)]
pub struct Tracked<T: Component> {
    t: T,
    tracker: Tracker,
}
impl<T: Component> Tracked<T> {
    /// Keep the `binding_name` [content](Content) binding in sync with `T`'s value.
    pub fn content(binding_name: &'static str, t: T) -> Self
    where
        T: fmt::Display,
    {
        let proto_fetch: ProtoFetch = |entity| some_content(entity.get::<T>()?);
        Self { t, tracker: Tracker { binding_name, proto_fetch } }
    }
    /// Keep the `binding_name` [content](Content) binding in sync with `T`'s debug value.
    ///
    /// Typically useful for debugging, as, unlike [`Tracked::content`],
    /// you can derive `Debug`. You may enable the `"no_tracked_debug"`
    /// `cuicui_richtext` crate feature to turns this into a no-op.
    #[cfg(not(feature = "no_tracked_debug"))]
    pub fn debug(binding_name: &'static str, t: T) -> Self
    where
        T: fmt::Debug,
    {
        let proto_fetch: ProtoFetch = |entity| some_content(format!("{:?}", entity.get::<T>()?));
        Self { t, tracker: Tracker { binding_name, proto_fetch } }
    }
    #[cfg(feature = "no_tracked_debug")]
    pub fn debug(_binding_name: &'static str, t: T) -> T {
        t
    }
    /// Keep `binding_name` binding in sync with `T`'s [`Modify`] value.
    pub fn modifier(binding_name: &'static str, t: T) -> Self
    where
        T: IntoModify + Clone,
    {
        let proto_fetch: ProtoFetch = |entity| Some(entity.get::<T>()?.clone().into_modify());
        Self { t, tracker: Tracker { binding_name, proto_fetch } }
    }
    // TODO(feat): a `Tracked` that accepts a Reflect + Path, as it allows tracking
    // stuff like `Transform` positions.
}

pub fn update_tracked_components(world: &mut World, mut entities: Local<Vec<Entity>>) {
    world.resource_scope(|world, mut world_bindings: Mut<WorldBindings>| {
        entities.extend(world.query_filtered::<Entity, With<Tracker>>().iter(world));
        for entity in entities.drain(..) {
            // TODO(perf): the 2 next Option<> can safely be unchecked_unwraped
            let Some(entity) = world.get_entity(entity) else { continue; };
            let Some(tracker) = entity.get::<Tracker>() else { continue; };
            let Some(modify) = (tracker.proto_fetch)(entity) else { continue; };
            world_bindings.set(tracker.binding_name, modify);
        }
    })
}
