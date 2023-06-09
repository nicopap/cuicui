use std::{fmt, marker::PhantomData};

use bevy::{ecs::world::EntityRef, prelude::*};
use fab::binding;

use crate::{BevyModify, WorldBindings};

/// Track a component of this entity and keep the binding `binding_name`
/// in [`WorldBindings`] up to date with its value.
#[derive(Component)]
struct Tracker<M> {
    // TODO(perf): use binding::Id instead here.
    binding_name: &'static str,
    proto_fetch: fn(EntityRef, binding::Entry<M>),
}

/// Add a tracked `T` which value will be kept in sync in [`WorldBindings`].
///
/// Use one of `TrackerBundle`'s method or the `track!` macro to create a tracked
/// component.
#[derive(Bundle)]
pub struct TrackerBundle<T: Component, M: 'static> {
    t: T,
    tracker: Tracker<M>,
}
impl<T: Component, M> TrackerBundle<T, M> {
    fn new(
        t: T,
        binding_name: &'static str,
        proto_fetch: fn(EntityRef, binding::Entry<M>),
    ) -> Self {
        Self { t, tracker: Tracker { binding_name, proto_fetch } }
    }
    /// Keep the `binding_name` [content] binding in sync with `T`'s value.
    ///
    /// [content]: BevyModify::set_content
    pub fn content(binding_name: &'static str, t: T) -> Self
    where
        T: fmt::Display,
        M: BevyModify,
    {
        Self::new(t, binding_name, |entity, entry| {
            let Some(s) = entity.get::<T>() else { return; };
            entry
                .modify(|m| m.set_content(format_args!("{s}")))
                .or_insert_with(|| M::init_content(format_args!("{s}")));
        })
    }
    /// Keep the `binding_name` [content] binding in sync with `T`'s debug value.
    ///
    /// Typically useful for debugging, as, unlike [`TrackerBundle::content`],
    /// you can derive `Debug`. You may enable the `"no_tracked_debug"`
    /// `cuicui_richtext` crate feature to turns this into a no-op.
    ///
    /// [content]: BevyModify::set_content
    #[cfg(not(feature = "no_tracked_debug"))]
    pub fn debug(binding_name: &'static str, t: T) -> Self
    where
        T: fmt::Debug,
        M: BevyModify,
    {
        Self::new(t, binding_name, |entity, entry| {
            let Some(s) = entity.get::<T>() else { return; };
            entry
                .modify(|m| m.set_content(format_args!("{s:?}")))
                .or_insert_with(|| M::init_content(format_args!("{s:?}")));
        })
    }
    #[cfg(feature = "no_tracked_debug")]
    pub fn debug(_binding_name: &'static str, t: T) -> T {
        t
    }
    /// Keep `binding_name` binding in sync with `T`'s [`Modify`] value.
    ///
    /// [`Modify`]: fab::Modify
    pub fn modifier(binding_name: &'static str, t: T) -> Self
    where
        M: for<'a> From<&'a T>,
    {
        Self::new(t, binding_name, |entity, entry| {
            let Some(s) = entity.get::<T>() else { return; };
            entry.insert(s.into());
        })
    }
}

#[derive(Component)]
pub struct TrackerBinding<M>(binding::Id, PhantomData<fn(M)>);
impl<M> Clone for TrackerBinding<M> {
    fn clone(&self) -> Self {
        TrackerBinding(self.0, PhantomData)
    }
}
type Q<M> = (Entity, Option<&'static TrackerBinding<M>>);

pub fn update_component_trackers_system<M: BevyModify>(
    world: &mut World,
    mut entities: Local<Vec<(Entity, Option<TrackerBinding<M>>)>>,
    mut cache_binding_id: Local<Vec<(Entity, binding::Id)>>,
) {
    world.resource_scope(|world, mut world_bindings: Mut<WorldBindings<M>>| {
        // We do this weird dance because we need to pass a EntityRef to proto_fetch
        entities.extend(
            world
                .query_filtered::<Q<M>, With<Tracker<M>>>()
                .iter(world)
                .map(|(e, id)| (e, id.cloned())),
        );
        // note that `drain(..)` also clears the vec for next call
        for (entity, binding) in entities.drain(..) {
            // SAFETY: all entity in entities has a Tracker component and exist.
            // because of that query_filtered
            let entity_ref = unsafe { world.get_entity(entity).unwrap_unchecked() };
            let tracker = unsafe { entity_ref.get::<Tracker<M>>().unwrap_unchecked() };
            let id = match binding {
                Some(binding) => binding.0,
                None => {
                    let id = world_bindings.bindings.get_or_add(tracker.binding_name);
                    cache_binding_id.push((entity, id));
                    id
                }
            };
            let entry = world_bindings.bindings.entry(id);
            (tracker.proto_fetch)(entity_ref, entry);
        }
    });
    // note that `drain(..)` also clears the vec for next call
    for (entity, to_insert) in cache_binding_id.drain(..) {
        world
            .entity_mut(entity)
            .insert(TrackerBinding::<M>(to_insert, PhantomData));
    }
}
