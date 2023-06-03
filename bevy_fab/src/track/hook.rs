use bevy::ecs::prelude::{Mut, Resource, World};
use fab::binding;

use super::{Read, Write};
use crate::{BevyModify, WorldBindings};

/// A hook from a value in the ECS to a [`M: Modify`] associated with
/// a binding.
///
/// In the format string, a hook is a special binding that declares a value
/// to read from the ECS and how to interpret it.
///
/// This typically looks like `{Res.ResourceType.field.to.value:formatting}`.
///
/// All hooks are added the the [`Hooks`] resource, see [`Hooks`] for more
/// details.
///
/// [`M: Modify`]: fab::modify::Modify
pub struct Hook<M> {
    binding: binding::Id,
    read: Read,
    write: Write<M>,
}
impl<M: BevyModify> Hook<M> {
    pub fn from_parsed(
        hook: fab_parse::tree::Hook,
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
    fn read_into_binding(
        &mut self,
        world: &World,
        bindings: &mut Mut<WorldBindings<M>>,
    ) -> Option<()> {
        let value = self.read.world(world)?;
        self.write.modify(value, bindings.0.entry(self.binding));
        Some(())
    }
}

/// The hooks to run.
///
/// In the format string, a hook is a special binding that declares a value
/// to read from the ECS and how to interpret it.
///
/// Hooks are added to this resource by [`parse_into_resolver_system`] and read by
/// [`update_hooked`] to update [`WorldBindings`] with the content of hooked values.
///
/// [`parse_into_resolver_system`]: crate::make::parse_into_resolver_system
#[derive(Resource)]
pub struct Hooks<M>(Vec<Hook<M>>); // TODO(clean): merge this with WorldBindings
impl<M> Default for Hooks<M> {
    fn default() -> Self {
        Hooks(Vec::new())
    }
}
impl<M> Hooks<M> {
    pub fn extend(&mut self, iter: impl IntoIterator<Item = Hook<M>>) {
        self.0.extend(iter)
    }
}
pub fn update_hooked<M: BevyModify>(world: &mut World) {
    world.resource_scope(|world, mut bindings: Mut<WorldBindings<M>>| {
        world.resource_scope(|world, mut trackers: Mut<Hooks<M>>| {
            for tracker in trackers.0.iter_mut() {
                tracker.read_into_binding(world, &mut bindings);
            }
        })
    })
}
