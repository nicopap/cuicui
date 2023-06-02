use std::{fmt, marker::PhantomData};

use bevy::{ecs::world::EntityRef, prelude::*};
use fab::{binding, prefab::Prefab};

use crate::{BevyPrefab, PrefabWorld};

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
/// Use one of `Tracked`'s method or the [`track!`] macro to create a tracked
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
    /// [content]: crate::modifiers::Content
    pub fn content(binding_name: &'static str, t: T) -> Self
    where
        T: fmt::Display,
        M: fmt::Write + From<String>,
    {
        Self::new(t, binding_name, |entity, entry| {
            let Some(s) = entity.get::<T>() else { return; };
            let write = |m: &mut M| {
                m.write_fmt(format_args!("{s}")).unwrap();
            };
            entry.modify(write).or_insert_with(|| s.to_string().into());
        })
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
        M: fmt::Write + From<String>,
    {
        Self::new(t, binding_name, |entity, entry| {
            let Some(s) = entity.get::<T>() else { return; };
            let write = |m: &mut M| {
                m.write_fmt(format_args!("{s:?}")).unwrap();
            };
            let debug_text = || M::from(format!("{s:?}"));
            entry.modify(write).or_insert_with(debug_text);
        })
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
type Q<P> = (
    Entity,
    Option<&'static TrackerBinding<<P as Prefab>::Modify>>,
);

pub fn update_component_trackers_system<P>(
    world: &mut World,
    mut entities: Local<Vec<(Entity, Option<TrackerBinding<P::Modify>>)>>,
    mut cache_binding_id: Local<Vec<(Entity, binding::Id)>>,
) where
    P: BevyPrefab + 'static,
    P::Modify: fmt::Write + From<String> + Send + Sync,
{
    world.resource_scope(|world, mut world_bindings: Mut<PrefabWorld<P>>| {
        // We do this weird dance because we need to pass a EntityRef to proto_fetch
        entities.extend(
            world
                .query_filtered::<Q<P>, With<Tracker<P::Modify>>>()
                .iter(world)
                .map(|(e, id)| (e, id.cloned())),
        );
        // note that `drain(..)` also clears the vec for next call
        for (entity, binding) in entities.drain(..) {
            // SAFETY: all entity in entities has a Tracker component and exist.
            // because of that query_filtered
            let entity_ref = unsafe { world.get_entity(entity).unwrap_unchecked() };
            let tracker = unsafe { entity_ref.get::<Tracker<P::Modify>>().unwrap_unchecked() };
            let id = match binding {
                Some(binding) => binding.0,
                None => {
                    let id = world_bindings.0.get_or_add(tracker.binding_name);
                    cache_binding_id.push((entity, id));
                    id
                }
            };
            let entry = world_bindings.0.entry(id);
            (tracker.proto_fetch)(entity_ref, entry);
        }
    });
    // note that `drain(..)` also clears the vec for next call
    for (entity, to_insert) in cache_binding_id.drain(..) {
        world
            .entity_mut(entity)
            .insert(TrackerBinding::<P::Modify>(to_insert, PhantomData));
    }
}