use std::ptr::NonNull;
use std::{any, fmt};

use bevy::app::AppTypeRegistry;
use bevy::core::Name;
use bevy::ecs::{
    prelude::*, query::QuerySingleError, reflect::ReflectComponentFns, reflect::ReflectResourceFns,
    world::EntityRef,
};
use bevy::reflect::{ParsedPath, Reflect, ReflectPathError, TypeData};
use fab_parse::tree as parse;
use reflect_query::{ReflectQueryable, ReflectQueryableFns, ReflectQueryableIterEntities};
use thiserror::Error;

pub type NewResult<T> = Result<T, ParseError>;
pub type GetResult<T> = Result<T, GetError>;

type FromEntity = fn(EntityRef<'_>) -> Option<&dyn Reflect>;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Couldn't parse the source path: {0}")]
    ReflectPath(String),

    #[error("`world` Doesn't contain the component in the binding source")]
    NotInWorld,

    #[error(transparent)]
    OneError(#[from] QuerySingleError),

    #[error("`world` has no type regsitry")]
    NoTypeRegistry,

    #[error("`{0}` type is not in the registry")]
    NotInRegistry(Box<str>),

    #[error("`{0}` type doesn't reflect {1}, add #[reflect({1})] to its definition")]
    NoTypeData(Box<str>, ReflectTrait),
}
impl From<ReflectPathError<'_>> for ParseError {
    fn from(value: ReflectPathError<'_>) -> Self {
        ParseError::ReflectPath(value.to_string())
    }
}

#[derive(Debug, Error)]
pub enum GetError {
    #[error("Entity previously matched doesn't contain the binding source component anymore")]
    NoComponent,

    #[error("Entity previously matched doesn't exist anymore")]
    NoEntity,

    #[error("Can't extract resource from world, it isn't in it")]
    NotInWorld,

    #[error("Couldn't parse the source path: {0}")]
    ReflectPath(String),
}
impl From<ReflectPathError<'_>> for GetError {
    fn from(value: ReflectPathError<'_>) -> Self {
        GetError::ReflectPath(value.to_string())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ReflectTrait {
    Component,
    Resource,
    Queryable,
}

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
    pub fn from_parsed(parsed: parse::Source, world: &mut World) -> NewResult<Self> {
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
        Ok(Read { query })
    }
    pub fn world<'a>(&mut self, world: &'a mut World) -> GetResult<&'a dyn Reflect> {
        match &self.query {
            Query::Res(access) => access.get(world),
            Query::One(access) => access.get(world),
            Query::Name(access) => access.get(world),
            Query::Marked(access) => access.get(world),
        }
    }
}

fn cast_to_resource_fns(reflect: &ReflectResource) -> ReflectResourceFns {
    // SAFETY: we are casting from `ReflectResource` to `ReflectResourceFns`
    // This is ill-advised, as ReflectResource isn't #[repr(transparent)],
    // however, this isn't unsound as of the current (1.69) version of rust,
    // since `ReflectResource` is a newtype for `ReflectResourceFns` and
    // guarentees to be no different.
    let new_ref: &ReflectResourceFns = unsafe { NonNull::from(reflect).cast().as_ref() };
    new_ref.clone()
}
fn cast_to_comp_fns(reflect: &ReflectComponent) -> ReflectComponentFns {
    // SAFETY: see above
    let new_ref: &ReflectComponentFns = unsafe { NonNull::from(reflect).cast().as_ref() };
    new_ref.clone()
}
fn get_queryable_fns(reflect: &ReflectQueryable) -> ReflectQueryableFns {
    reflect.get().clone()
}

fn get_path(path: &str) -> NewResult<Option<ParsedPath>> {
    match path {
        "" => Ok(None),
        path => Ok(Some(ParsedPath::parse(path)?)),
    }
}
fn read_path<'a>(
    path: &Option<ParsedPath>,
    reflect: &'a dyn Reflect,
) -> GetResult<&'a dyn Reflect> {
    match path {
        None => Ok(reflect),
        Some(path) => Ok(path.reflect_element(reflect)?),
    }
}
fn get_data<T: TypeData, Out>(
    world: &World,
    type_name: &str,
    f: impl FnOnce(&T) -> Out,
) -> NewResult<Out> {
    let no_type_data = || {
        let type_name = any::type_name::<T>();
        let reflect_trait = match () {
            () if type_name.ends_with("Component") => ReflectTrait::Component,
            () if type_name.ends_with("Resource") => ReflectTrait::Resource,
            () if type_name.ends_with("Queryable") => ReflectTrait::Queryable,
            () => unreachable!("Trait no one cares about"),
        };
        ParseError::NoTypeData(type_name.into(), reflect_trait)
    };
    Ok(f(world
        .get_resource::<AppTypeRegistry>()
        .ok_or(ParseError::NoTypeRegistry)?
        .read()
        .get_with_short_name(type_name)
        .ok_or_else(|| ParseError::NotInRegistry(type_name.into()))?
        .data()
        .ok_or_else(no_type_data)?))
}
/// iterates over all entities in the world in research for the one matched by `from_entity`.
///
/// This will, of course, iterate over all entities if `from_entity` doesn't match
/// anything. So use sparingly.
fn get_first_entity(world: &World, matches: impl Fn(EntityRef) -> bool) -> NewResult<Entity> {
    world
        .iter_entities()
        .find_map(|e| matches(e).then_some(e.id()))
        .ok_or(ParseError::NotInWorld)
}

#[derive(Clone)]
pub(crate) struct ResAccess {
    from_world: fn(&World) -> Option<&dyn Reflect>,
    path: Option<ParsedPath>,
}
// TODO(perf): consider storing a pointer offset and cast from `Ptr<'a, _>`
// taken from `World::get_*_by_id(ComponentId) + offset` instead of Option<ParsedPath>
#[derive(Clone)]
pub(crate) struct OneAccess {
    entity: Entity,
    from_entity: FromEntity,
    #[allow(unused)] // TODO(feat): react to when world.get(entity) -> None
    get_entity: fn(&mut World) -> Result<Entity, QuerySingleError>,
    path: Option<ParsedPath>,
}
#[derive(Clone)]
pub(crate) struct MarkedAccess {
    entity: Entity,
    from_entity: FromEntity,
    #[allow(unused)] // TODO(feat): react to when world.get(entity) -> None
    get_entities: fn(&mut World) -> ReflectQueryableIterEntities,
    path: Option<ParsedPath>,
}
#[derive(Clone)]
pub(crate) struct NameAccess {
    entity: Entity,
    from_entity: FromEntity,
    path: Option<ParsedPath>,
    #[allow(unused)] // TODO(feat): react to when world.get(entity) -> None
    name: Name,
}
impl ResAccess {
    // TODO(bug): update Access when world changes
    fn get<'a>(&self, world: &'a World) -> GetResult<&'a dyn Reflect> {
        let resource = (self.from_world)(world).ok_or(GetError::NotInWorld)?;
        read_path(&self.path, resource)
    }
    fn new(type_name: &str, path: &str, world: &World) -> NewResult<Self> {
        let from_world = get_data(world, type_name, cast_to_resource_fns)?.reflect;
        let path = get_path(path)?;
        Ok(ResAccess { from_world, path })
    }
}
impl OneAccess {
    fn get<'a>(&self, world: &'a World) -> GetResult<&'a dyn Reflect> {
        let entity = world.get_entity(self.entity).ok_or(GetError::NoEntity)?;
        let entity = (self.from_entity)(entity).ok_or(GetError::NoComponent)?;
        read_path(&self.path, entity)
    }
    fn new(one: &str, path: &str, world: &mut World) -> NewResult<Self> {
        let from_entity = get_data(world, one, cast_to_comp_fns)?.reflect;
        let get_entity = get_data(world, one, get_queryable_fns)?.get_single_entity;
        let path = get_path(path)?;
        let entity = get_entity(world)?;
        Ok(OneAccess { from_entity, get_entity, entity, path })
    }
}
impl MarkedAccess {
    fn get<'a>(&self, world: &'a World) -> GetResult<&'a dyn Reflect> {
        let entity = world.get_entity(self.entity).ok_or(GetError::NoEntity)?;
        let entity = (self.from_entity)(entity).ok_or(GetError::NoComponent)?;
        read_path(&self.path, entity)
    }
    fn new(marker: &str, accessed: &str, path: &str, world: &mut World) -> NewResult<Self> {
        let from_entity = get_data(world, accessed, cast_to_comp_fns)?.reflect;
        let get_entities = get_data(world, marker, get_queryable_fns)?.iter_entities;
        let path = get_path(path)?;
        let entity = get_entities(world).iter(world).next();
        let entity = entity.ok_or(ParseError::NotInWorld)?;
        Ok(MarkedAccess { from_entity, get_entities, entity, path })
    }
}
impl NameAccess {
    fn get<'a>(&self, world: &'a World) -> GetResult<&'a dyn Reflect> {
        let entity = world.get_entity(self.entity).ok_or(GetError::NoEntity)?;
        let entity = (self.from_entity)(entity).ok_or(GetError::NoComponent)?;
        read_path(&self.path, entity)
    }
    fn new(name: String, accessed: &str, path: &str, world: &World) -> NewResult<Self> {
        let name = name.into();
        let from_entity = get_data(world, accessed, cast_to_comp_fns)?.reflect;
        let path = get_path(path)?;
        let entity = get_first_entity(world, |e| e.get::<Name>() == Some(&name))?;
        Ok(NameAccess { from_entity, name, entity, path })
    }
}

//
// fmt::Display impls
//

impl fmt::Display for ResAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Res.<accessed_type>")?;
        if let Some(path) = &self.path {
            write!(f, ".{}", path)?;
        }
        Ok(())
    }
}
impl fmt::Display for OneAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "One(<marker/accessed_type>)")?;
        if let Some(path) = &self.path {
            write!(f, ".{}", path)?;
        }
        Ok(())
    }
}
impl fmt::Display for NameAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name({})", self.name.as_str())?;
        if let Some(path) = &self.path {
            write!(f, ".{}", path)?;
        }
        Ok(())
    }
}
impl fmt::Display for MarkedAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Marked(<marker_type>).<accessed_type>")?;
        if let Some(path) = &self.path {
            write!(f, ".{}", path)?;
        }
        Ok(())
    }
}
impl fmt::Display for ReflectTrait {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReflectTrait::Component => write!(f, "Component"),
            ReflectTrait::Resource => write!(f, "Resource"),
            ReflectTrait::Queryable => write!(f, "Queryable"),
        }
    }
}
