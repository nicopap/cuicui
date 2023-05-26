//! Declare from format string what resource and components to read

mod parse;

use std::ptr::NonNull;

use bevy::{
    ecs::reflect::ReflectResourceFns, ecs::world::EntityRef, prelude::*, reflect::ParsedPath,
    utils::HashMap,
};
use winnow::error::Error;

/// Where to pull from the value.
#[derive(Clone, Copy, Debug)]
enum RefNamespace<'a> {
    /// A [`Resource`] implementing [`Reflect`].
    Res(&'a str),
    /// The first [`Entity`] found with the given name
    Name { name: &'a str, access: &'a str },
    /// The first [`Entity`] found with provided component.
    One(&'a str),
    /// The first [`Entity`] found with provided component, but access a
    /// different component.
    Marked { marker: &'a str, access: &'a str },
}
enum Namespace {
    Res(Box<str>),
    Name { name: Box<str>, access: Box<str> },
    One(Box<str>),
    Marked { marker: Box<str>, access: Box<str> },
}
#[derive(Clone, Copy, Debug)]
struct RefTarget<'a> {
    namespace: RefNamespace<'a>,
    path: &'a str,
}
pub(crate) struct Target {
    namespace: Namespace,
    path: Box<str>,
}
impl Target {
    pub(crate) fn statik(input: &str) -> Option<Target> {
        RefTarget::parse(input).ok().map(|t| t.owned())
    }
    // TODO(err): cleaner error handling here, need to distinguish between:
    // - `reflect_path` gets a bad value
    // - `world` has no type regsitry
    // - `type_name` is not in the registry
    // - can't extract resource from world
    // - The resource hasn't changed since last frame.
    pub(crate) fn get<'a>(&self, cache: &mut Access, world: &'a World) -> Option<&'a dyn Reflect> {
        use Namespace as Ns;
        match &self.namespace {
            Ns::Res(type_name) => cache.get_res(type_name, &self.path, world),
            Ns::Name { .. } => todo!(),
            Ns::One(..) => todo!(),
            Ns::Marked { .. } => todo!(),
        }
    }
}
impl<'a> RefTarget<'a> {
    fn owned(&self) -> Target {
        use Namespace::*;
        use RefNamespace as Ref;

        let b = |s: &str| s.to_owned().into_boxed_str();
        Target {
            path: b(self.path),
            namespace: match self.namespace {
                Ref::Res(res) => Res(b(res)),
                Ref::One(one) => One(b(one)),
                Ref::Name { name, access } => Name { name: b(name), access: b(access) },
                Ref::Marked { marker, access } => Marked { marker: b(marker), access: b(access) },
            },
        }
    }
    pub(crate) fn parse(input: &'a str) -> Result<RefTarget, Error<&str>> {
        parse::target(input)
    }
}
struct ResAccess {
    from_world: fn(&World) -> Option<&dyn Reflect>,
    path: Option<ParsedPath>,
}
impl ResAccess {
    // TODO(err): have nice finer-grained errors with Result
    fn new(type_name: &str, path: &str, world: &World) -> Option<Self> {
        let registry = world.get_resource::<AppTypeRegistry>()?.read();
        let mk_reflect: &ReflectResource = registry.get_with_short_name(type_name)?.data()?;

        // SAFETY: we are casting from `ReflectResource` to `ReflectResourceFns`
        // This is ill-advised, as ReflectResource isn't #[repr(transparent)],
        // however, this isn't unsound as of the current (1.69) version of rust,
        // since `ReflectResource` is a newtype for `ReflectResourceFns` and
        // guarentees to be no different.
        let from_world = unsafe {
            let mk_reflect = NonNull::from(mk_reflect).cast::<ReflectResourceFns>();
            mk_reflect.as_ref().reflect
        };
        let path = match path {
            "" => None,
            path => Some(ParsedPath::parse(path).ok()?),
        };
        Some(ResAccess { from_world, path })
    }
    fn get<'a>(&self, world: &'a World) -> Option<&'a dyn Reflect> {
        let resource = (self.from_world)(world)?;
        if let Some(path) = &self.path {
            path.reflect_element(resource).ok()
        } else {
            Some(resource)
        }
    }
}
struct CompAccess {
    entity: Entity,
    from_entity: fn(EntityRef) -> Option<&dyn Reflect>,
    path: Option<ParsedPath>,
}
impl CompAccess {
    fn get<'a>(&self, world: &'a World) -> Option<&'a dyn Reflect> {
        let entity = world.get_entity(self.entity)?;
        let entity = (self.from_entity)(entity)?;
        if let Some(path) = &self.path {
            path.reflect_element(entity).ok()
        } else {
            Some(entity)
        }
    }
}

#[derive(Default)]
pub(crate) struct Access {
    // TODO: use interner instead.
    resources: HashMap<Box<str>, ResAccess>,
    components: HashMap<Box<str>, CompAccess>,
}
impl Access {
    fn get_res<'a>(
        &mut self,
        type_name: &str,
        path: &str,
        world: &'a World,
    ) -> Option<&'a dyn Reflect> {
        let already_cached = self.resources.contains_key(type_name);
        if already_cached {
            let access = self.resources.get(type_name).unwrap();
            access.get(world)
        } else {
            let access = ResAccess::new(type_name, path, world).unwrap();
            let result = access.get(world);
            self.resources.insert(type_name.into(), access);
            result
        }
    }
}
