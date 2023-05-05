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
/// - `track!('X, <identifier>, <expression>)`
/// - `track!('X, <identifier>)`
/// - `track!(<identifier>, <expression>)`
/// - `track!(<identifier>)`
///
/// The first (optional) parameter is a 'label. It can have one of
/// the following values:
///
/// - `'c` (default if omitted): See [`Tracked::content`].
///   Track the component as a content binding.
///   This requires the component to implement [`fmt::Display`].
/// - `'d`: See [`Tracked::debug`]. Track the component as a debug content binding.
///   This requires the component to implement [`fmt::Debug`].
///   If the cargo feature `no_tracked_debug` is enabled, it will just insert the component.
/// - `'m`: See [`Tracked::modifier`].
///   Track the component as a modifier binding.
///   It must implement [`IntoModify`] and [`Clone`].
///
/// The second parameter is a rust identifier. It will be the name of the binding
/// in [`WorldBindings`] the component will have.
///
/// The third parameter is the component as it should be added to the entity.
/// If omitted, `track!` will use the second parameters value instead, assuming
/// that it is a variable containing the component as value.
///
/// ## Examples
///
/// ```
/// use bevy::prelude::*;
/// use cuicui_richtext::track;
///
/// # #[derive(Component, Default)] struct MaxValue(f32);
/// # #[derive(Component, Default)] struct MinValue(f32);
/// # #[derive(Component, Default, Reflect, Debug)]
/// # struct Slider(f32);
/// #
/// # #[derive(Bundle, Default)]
/// # struct NonsliderBundle {
/// #     max: MaxValue,
/// #     min: MinValue,
/// # }
/// fn setup(mut commands: Commands) {
///     let value = 11.0;
///     commands.spawn((
///         NonsliderBundle { max: MaxValue(34.0), ..default() },
///         track!('d, slider, Slider(value)),
///     ));
/// }
/// ```
///
/// Note that if the component you want to track is already part of a bundle,
/// you can work around it by inserting the tracked version afterward.
///
/// ```
/// use bevy::prelude::*;
/// use cuicui_richtext::track;
///
/// # #[derive(Component, Default)] struct MaxValue(f32);
/// # #[derive(Component, Default)] struct MinValue(f32);
/// # #[derive(Component, Default, Reflect, Debug)]
/// # struct Slider(f32);
/// # impl std::fmt::Display for Slider {
/// #    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:.3}", self.0) }
/// # }
/// #
/// # #[derive(Bundle, Default)]
/// # struct SliderBundle {
/// #     max: MaxValue,
/// #     min: MinValue,
/// #     slider: Slider,
/// # }
/// # fn setup(mut commands: Commands) {
/// let value = 11.0;
/// commands.spawn(
///     SliderBundle { max: MaxValue(100.0), slider: Slider(value), ..default() },
/// );
/// // Becomes:
/// commands.spawn(
///     SliderBundle { max: MaxValue(100.0), ..default() }
/// )
/// .insert(track!(slider, Slider(value)));
/// # }
/// ```
///
/// ## Implementation notes
///
/// The macro accepts a lifetime (`'d`) as first parameter because it's the only
/// thing that can be matched by name in macro expansion, outside of an identifier
/// and a generic token. But identifier is already used in this position, so
/// we chose a lifetime (I would have rather used a string literal).
#[macro_export]
macro_rules! track {
    (@ctor 'd) => { $crate::Tracked::debug };
    (@ctor 'm) => { $crate::Tracked::modifier };
    (@ctor 'c) => { $crate::Tracked::content };
    (@ctor ) => { $crate::Tracked::content };
    (@build($ctor:expr, $binding_name:expr, $component:expr)) => {
        ($ctor)($binding_name, $component)
    };
    ($( $flag:lifetime, )? $binding_name:ident , $component:expr)  => {{
        let ctor = track!(@ctor $($flag)?);
        track!(@build(ctor, stringify!($binding_name), $component))
    }};
    ($( $flag:lifetime, )? $variable_name:ident)  => {
        let ctor = track!(@ctor $($flag)?);
        track!(@build(ctor, stringify!($variable_name), $variable_name))
    };
}

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
    /// Keep the `binding_name` [content] binding in sync with `T`'s value.
    ///
    /// [content]: crate::modifiers::Content
    pub fn content(binding_name: &'static str, t: T) -> Self
    where
        T: fmt::Display,
    {
        let proto_fetch: ProtoFetch = |entity| some_content(entity.get::<T>()?);
        Self { t, tracker: Tracker { binding_name, proto_fetch } }
    }
    /// Keep the `binding_name` [content] binding in sync with `T`'s debug value.
    ///
    /// Typically useful for debugging, as, unlike [`Tracked::content`],
    /// you can derive `Debug`. You may enable the `"no_tracked_debug"`
    /// `cuicui_richtext` crate feature to turns this into a no-op.
    ///
    /// [content]: crate::modifiers::Content
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
    ///
    /// [`Modify`]: crate::Modify
    pub fn modifier(binding_name: &'static str, t: T) -> Self
    where
        T: IntoModify + Clone,
    {
        let proto_fetch: ProtoFetch = |entity| Some(entity.get::<T>()?.clone().into_modify());
        Self { t, tracker: Tracker { binding_name, proto_fetch } }
    }
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
