use std::ptr::NonNull;

use bevy::app::AppTypeRegistry;
use bevy::core::Name;
use bevy::ecs::{
    prelude::*, reflect::ReflectComponentFns, reflect::ReflectResourceFns, world::EntityRef,
};
use bevy::reflect::{ParsedPath, Reflect};
use fab_parse::tree as parse;

#[derive(Clone)]
pub(crate) enum Query {
    Res(ResAccess),
    Name(NameAccess),
    One(OneAccess),
    Marked(MarkedAccess),
}

/// Read a value from the [`World`], through [`Read::world`], returning a [`&dyn Reflect`].
pub struct Read {
    query: Query,
}

impl Read {
    // TODO(err)
    pub(crate) fn from_parsed(parsed: parse::Source, world: &mut World) -> Option<Self> {
        let path: Box<str> = parsed.reflect_path.into();
        let query = match parsed.query {
            parse::Query::Res(res) => Query::Res(ResAccess::new(res, &path, world)?),
            parse::Query::Name { name, access } => {
                Query::Name(NameAccess::new(name.into(), access, &path, world)?)
            }
            parse::Query::One(one) => Query::One(OneAccess::new(one, &path, world)?),
            parse::Query::Marked { marker, access } => {
                Query::Marked(MarkedAccess::new(marker, access, &path, world)?)
            }
        };
        Some(Read { query })
    }
    // TODO(err): cleaner error handling here, need to distinguish between:
    // - `reflect_path` gets a bad value
    // - `world` has no type regsitry
    // - `type_name` is not in the registry
    // - can't extract resource from world
    // - The resource hasn't changed since last frame.
    pub(crate) fn world<'a>(&mut self, world: &'a World) -> Option<&'a dyn Reflect> {
        match &self.query {
            Query::Res(access) => access.get(world),
            Query::One(access) => access.get(world),
            Query::Name(access) => access.get(world),
            Query::Marked(access) => access.get(world),
        }
    }
}

fn cast_to_resource_fns(reflect_resource: &ReflectResource) -> &ReflectResourceFns {
    // SAFETY: we are casting from `ReflectResource` to `ReflectResourceFns`
    // This is ill-advised, as ReflectResource isn't #[repr(transparent)],
    // however, this isn't unsound as of the current (1.69) version of rust,
    // since `ReflectResource` is a newtype for `ReflectResourceFns` and
    // guarentees to be no different.
    unsafe { NonNull::from(reflect_resource).cast().as_ref() }
}
fn cast_to_comp_fns(reflect_component: &ReflectComponent) -> &ReflectComponentFns {
    // SAFETY: we are casting from `ReflectResource` to `ReflectResourceFns`
    // This is ill-advised, as ReflectResource isn't #[repr(transparent)],
    // however, this isn't unsound as of the current (1.69) version of rust,
    // since `ReflectResource` is a newtype for `ReflectResourceFns` and
    // guarentees to be no different.
    unsafe { NonNull::from(reflect_component).cast().as_ref() }
}

#[derive(Clone)]
pub(crate) struct ResAccess {
    from_world: fn(&World) -> Option<&dyn Reflect>,
    path: Option<ParsedPath>,
}
impl ResAccess {
    // TODO(err): have nice finer-grained errors with Result
    pub(crate) fn new(type_name: &str, path: &str, world: &World) -> Option<Self> {
        let registry = world.get_resource::<AppTypeRegistry>()?.read();
        let mk_reflect: &ReflectResource = registry.get_with_short_name(type_name)?.data()?;

        let from_world = cast_to_resource_fns(mk_reflect).reflect;
        let path = match path {
            "" => None,
            path => Some(ParsedPath::parse(path).ok()?),
        };
        Some(ResAccess { from_world, path })
    }
    // TODO(bug): update Access when world changes
    fn get<'a>(&self, world: &'a World) -> Option<&'a dyn Reflect> {
        let resource = (self.from_world)(world)?;
        if let Some(path) = &self.path {
            path.reflect_element(resource).ok()
        } else {
            Some(resource)
        }
    }
}
// TODO(perf): consider storing a pointer offset and cast from `Ptr<'a, _>`
// taken from `World::get_*_by_id(ComponentId) + offset` instead of Option<ParsedPath>
#[derive(Clone)]
pub(crate) struct OneAccess {
    entity: Entity,
    from_entity: fn(EntityRef) -> Option<&dyn Reflect>,
    path: Option<ParsedPath>,
}
impl OneAccess {
    fn get<'a>(&self, world: &'a World) -> Option<&'a dyn Reflect> {
        let entity = world.get_entity(self.entity)?;
        let entity = (self.from_entity)(entity)?;
        if let Some(path) = &self.path {
            path.reflect_element(entity).ok()
        } else {
            Some(entity)
        }
    }

    pub(crate) fn new(one: &str, path: &str, world: &World) -> Option<Self> {
        let registry = world.get_resource::<AppTypeRegistry>()?.read();

        let reflect_one = cast_to_comp_fns(registry.get_with_short_name(one)?.data()?);
        let (from_entity, contains) = (reflect_one.reflect, reflect_one.contains);

        let path = match path {
            "" => None,
            path => Some(ParsedPath::parse(path).ok()?),
        };
        // This is extremelymely costly, only do that when absolutely necessary.
        let entity = world.iter_entities().find(|e| contains(*e))?.id();
        Some(OneAccess { from_entity, entity, path })
    }
}
#[derive(Clone)]
pub(crate) struct NameAccess {
    entity: Entity,
    from_entity: fn(EntityRef) -> Option<&dyn Reflect>,
    path: Option<ParsedPath>,
    #[allow(unused)] // TODO(feat): react to when world.get(entity) -> None
    name: Name,
}
impl NameAccess {
    fn get<'a>(&self, world: &'a World) -> Option<&'a dyn Reflect> {
        let entity = world.get_entity(self.entity)?;
        let entity = (self.from_entity)(entity)?;
        if let Some(path) = &self.path {
            path.reflect_element(entity).ok()
        } else {
            Some(entity)
        }
    }

    pub(crate) fn new(name: String, accessed: &str, path: &str, world: &World) -> Option<Self> {
        let name = name.into();
        let registry = world.get_resource::<AppTypeRegistry>()?.read();

        let from_entity = cast_to_comp_fns(registry.get_with_short_name(accessed)?.data()?).reflect;
        let contains = |e: &EntityRef| e.get::<Name>() == Some(&name);

        let path = match path {
            "" => None,
            path => Some(ParsedPath::parse(path).ok()?),
        };
        // This is extremelymely costly, only do that when absolutely necessary.
        let entity = world.iter_entities().find(contains)?.id();
        Some(NameAccess { from_entity, name, entity, path })
    }
}
#[derive(Clone)]
pub(crate) struct MarkedAccess {
    entity: Entity,
    from_entity: fn(EntityRef) -> Option<&dyn Reflect>,
    #[allow(unused)] // TODO(feat): react to when world.get(entity) -> None
    contains: fn(EntityRef) -> bool,
    path: Option<ParsedPath>,
}
impl MarkedAccess {
    fn get<'a>(&self, world: &'a World) -> Option<&'a dyn Reflect> {
        let entity = world.get_entity(self.entity)?;
        let entity = (self.from_entity)(entity)?;
        if let Some(path) = &self.path {
            path.reflect_element(entity).ok()
        } else {
            Some(entity)
        }
    }

    pub(crate) fn new(marker: &str, accessed: &str, path: &str, world: &World) -> Option<Self> {
        let registry = world.get_resource::<AppTypeRegistry>()?.read();

        let from_entity = cast_to_comp_fns(registry.get_with_short_name(accessed)?.data()?).reflect;
        let contains = cast_to_comp_fns(registry.get_with_short_name(marker)?.data()?).contains;

        let path = match path {
            "" => None,
            path => Some(ParsedPath::parse(path).ok()?),
        };
        // This is extremelymely costly, only do that when absolutely necessary.
        let entity = world.iter_entities().find(|e| contains(*e))?;
        let entity = entity.id();
        Some(MarkedAccess { from_entity, contains, entity, path })
    }
}
