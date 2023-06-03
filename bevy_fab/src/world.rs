//! Global world-scopped data relevant to [`Modify`]s located in the bevy ECS.
use bevy::prelude::{error, Mut, Resource, World};

use fab::binding;
use fab_parse::tree::Hook as ParsedHook;

use crate::track::{Read, Write};
use crate::BevyModify;

/// A hook from a value in the ECS to a [`M: Modify`] associated with
/// a binding.
///
/// In the format string, a hook is a special binding that declares a value
/// to read from the ECS and how to interpret it.
///
/// This typically looks like `{Res.ResourceType.field.to.value:formatting}`.
///
/// All hooks are added the the [`WorldBindings`] resource.
///
/// [`M: Modify`]: fab::modify::Modify
pub struct Hook<M> {
    binding: binding::Id,
    read: Read,
    write: Write<M>,
}
impl<M: BevyModify> Hook<M> {
    fn from_parsed(
        hook: ParsedHook,
        world: &mut World,
        intern: impl FnOnce(&str) -> binding::Id,
    ) -> Option<Self> {
        Some(Hook {
            binding: intern(hook.source.binding),
            read: Read::from_parsed(hook.source, world)?,
            write: Write::from_parsed(hook.format),
        })
    }

    /// Read value describe in `self.read` from [`World`],
    /// then write it into binding `self.binding` in [`WorldBindings`]
    /// according to `self.write`.
    ///
    /// Note: `self` is mutable here, this is because [`Read`] caches world
    /// access to later access the value it reads much faster.
    fn read_into_binding(&mut self, world: &World, bindings: &mut binding::World<M>) -> Option<()> {
        let value = self.read.world(world)?;
        self.write.modify(value, bindings.entry(self.binding));
        Some(())
    }
}

/// The binding for all [`M: BevyModify`] in the ECS, and the hooks used by those.
///
/// In the format string, a hook is a special binding that declares a value
/// to read from the ECS and how to interpret it.
///
/// Hooks are added to this resource by [`parse_into_resolver_system`] and read by
/// [`update_hooked`] to update [`WorldBindings`] with the content of hooked values.
///
/// [`parse_into_resolver_system`]: crate::make::parse_into_resolver_system
/// [`M: BevyModify`]: BevyModify
#[derive(Resource)]
pub struct WorldBindings<M> {
    pub bindings: binding::World<M>,
    hooks: Vec<Hook<M>>,
}
impl<M> Default for WorldBindings<M> {
    fn default() -> Self {
        WorldBindings { bindings: Default::default(), hooks: Vec::new() }
    }
}
impl<M: BevyModify> WorldBindings<M> {
    pub fn add_hooks(&mut self, iter: impl IntoIterator<Item = Hook<M>>) {
        self.hooks.extend(iter)
    }
    pub fn parse_hook(&mut self, hook: ParsedHook, world: &mut World) {
        let Self { bindings, hooks } = self;
        if let Some(hook) = Hook::from_parsed(hook, world, |n| bindings.get_or_add(n)) {
            hooks.push(hook);
        } else {
            error!("A tracker failed to be loaded");
        }
    }
}

/// Update [`M::Items`] components co-located with [`LocalBindings`] that declare
/// a reflection-based dependency (`Res.foo.bar`, `One(MarkerComp).path`, etc)
/// when that dependency is updated.
///
/// [`M::Items`]: fab::modify::Modify::Items
/// [`LocalBindings`]: crate::LocalBindings
pub fn update_hooked<M: BevyModify>(world: &mut World) {
    world.resource_scope(|world, mut bindings: Mut<WorldBindings<M>>| {
        let WorldBindings { bindings, hooks } = &mut *bindings;
        for hook in hooks.iter_mut() {
            hook.read_into_binding(world, bindings);
        }
    })
}
