//! Tracker structs to easily insert into ECS components you want to read
//! into rich text modifiers.

mod component;

mod read;
mod write;

use bevy::prelude::{Mut, Resource, World};
use fab::binding;

use crate::{parse, richtext::WorldBindings};

pub use read::Read;
pub use write::Write;

pub use component::{update_tracked_components, Tracked};

/// A hook from a value in the ECS to a [`TextModifier`] associated with
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
/// [`TextModifier`]: crate::modifiers::TextModifier
pub struct Hook {
    binding: binding::Id,
    read: Read,
    write: Write,
}
impl Hook {
    pub fn from_parsed(
        hook: parse::Hook,
        bindings: &mut WorldBindings,
        world: &mut World,
    ) -> Option<Hook> {
        Some(Hook {
            binding: bindings.0.get_or_add(hook.source.binding),
            read: read::Read::from_parsed(hook.source, world)?,
            write: write::Write::from_parsed(hook.format),
        })
    }

    fn read_into_binding(
        &mut self,
        world: &World,
        bindings: &mut Mut<WorldBindings>,
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
/// Hooks are added to this resource by [`mk_richtext`] and read by
/// [`update_hooked`] to update [`WorldBindings`] with the
/// content of hooked values.
///
/// [`mk_richtext`]: crate::mk_richtext
#[derive(Resource, Default)]
pub struct Hooks(Vec<Hook>); // TODO(clean): merge this with WorldBindings
impl Hooks {
    pub fn extend(&mut self, iter: impl IntoIterator<Item = Hook>) {
        self.0.extend(iter)
    }
}
pub fn update_hooked(world: &mut World) {
    world.resource_scope(|world, mut bindings: Mut<WorldBindings>| {
        world.resource_scope(|world, mut trackers: Mut<Hooks>| {
            for tracker in trackers.0.iter_mut() {
                tracker.read_into_binding(world, &mut bindings);
            }
        })
    })
}
