//! Declare from format string what resource and components to read

use std::str::FromStr;

use bevy::prelude::{AppTypeRegistry, ReflectResource, World};
use bevy::reflect::{GetPath, Reflect};

/// Where to pull from the value.
#[derive(Copy, Clone, Debug)]
enum Namespace {
    /// A [`Resource`] implementing [`Reflect`].
    Res,
}
impl FromStr for Namespace {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        (s == "Res").then_some(Namespace::Res).ok_or(())
    }
}
#[derive(Copy, Clone, Debug)]
struct Path<'a> {
    path: &'a str,
}
#[derive(Copy, Clone, Debug)]
pub(crate) struct Target<'a> {
    _namespace: Namespace,
    path: Path<'a>,
}
impl<'a> Target<'a> {
    pub(crate) fn parse(input: &'a str) -> Option<Self> {
        let (namespace, path) = input.split_once('.')?;
        let _namespace = namespace.parse().ok()?;
        Some(Target { _namespace, path: Path { path } })
    }
    // TODO(err): cleaner error handling here, need to distinguish between:
    // - `reflect_path` gets a bad value
    // - `world` has no type regsitry
    // - `type_name` is not in the registry
    // - can't extract resource from world
    // - The resource hasn't changed since last frame.
    pub(crate) fn get<'b>(&self, world: &'b World) -> Option<&'b dyn Reflect> {
        let Path { path, .. } = self.path;
        let type_name = path.split_once('.').map_or(path, |p| p.0);

        // SAFETY: `type_name` is at most the same length as `path`.
        let path = unsafe { path.get_unchecked(type_name.len()..) };

        let registry = world.get_resource::<AppTypeRegistry>()?.read();
        let mk_reflect: &ReflectResource = registry.get_with_short_name(type_name)?.data()?;
        // TODO(perf): use `reflect_mut_unchecked` to get a Mut<dyn Reflect>
        // so as to be able to read is_changed
        let reflect = mk_reflect.reflect(world)?;
        match path {
            "" => Some(reflect),
            path => reflect.reflect_path(path).ok(),
        }
    }
}
