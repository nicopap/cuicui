use std::ptr::NonNull;
use std::{any, fmt};

use bevy::app::AppTypeRegistry;
use bevy::core::Name;
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::ecs::world::EntityRef;
use bevy::ecs::{prelude::*, query::QuerySingleError, reflect::ReflectResourceFns};
use bevy::reflect::{ParsedPath, Reflect, ReflectPathError, TypeData};
use fab_parse::tree as parse;
use reflect_query::{Ref, ReflectQueryable, ReflectQueryableFns};
use thiserror::Error;

pub type NewResult<T> = Result<T, ParseError>;
pub type GetResult<T> = Result<T, GetError>;

type ReflectMut = unsafe fn(UnsafeWorldCell) -> Option<Mut<dyn Reflect>>;
type FromEntity = fn(EntityRef) -> Option<Ref<dyn Reflect>>;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Couldn't parse the source path: {0}")]
    ReflectPath(String),

    #[error("`world` Doesn't contain an entity with name `{0}`")]
    NotInWorld(String),

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

    #[error(transparent)]
    OneError(#[from] QuerySingleError),

    /// This only happens if the path contains an index dereference (such as `["foo"]` or `[0]`)
    /// and the value of the reflected component changed between creation and querying.
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
    pub fn world<'a>(&mut self, world: &'a mut World) -> GetResult<Ref<'a, dyn Reflect>> {
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
pub fn reflect_ref(f: ReflectMut, world: &mut World) -> Option<Ref<dyn Reflect>> {
    // SAFETY: unique world access
    let reflect_mut = unsafe { f(world.as_unsafe_world_cell()) };
    reflect_mut.map(|r| Ref::map_from(r.into(), |i| i))
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
fn read_opt_path<'a, 'p>(
    path: &'p Option<ParsedPath>,
    reflect: &'a dyn Reflect,
) -> Result<&'a dyn Reflect, ReflectPathError<'p>> {
    match path {
        None => Ok(reflect),
        Some(path) => path.reflect_element(reflect),
    }
}
fn read_path<'a>(
    path: &Option<ParsedPath>,
    reflect: Ref<'a, dyn Reflect>,
) -> GetResult<Ref<'a, dyn Reflect>> {
    Ok(reflect.map_failable(|r| read_opt_path(path, r))?)
}
fn get_data<T: TypeData, Out>(
    world: &World,
    type_name: &str,
    f: impl FnOnce(&T) -> Out,
) -> NewResult<Out> {
    let no_type_data = || {
        let reflect_trait = match any::type_name::<T>() {
            td_name if td_name.ends_with("Component") => ReflectTrait::Component,
            td_name if td_name.ends_with("Resource") => ReflectTrait::Resource,
            td_name if td_name.ends_with("Queryable") => ReflectTrait::Queryable,
            td_name => unreachable!("Trait {td_name} no one cares about"),
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
/// Iterates over all entities with a `Name` component to find if one matches
/// the provided name.
///
/// Returns the first encountered `Entity` with given `name`.
/// `None` if no such thing exist.
fn get_with_name(world: &mut World, name: &Name) -> Option<Entity> {
    world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find_map(|(e, n)| (name == n).then_some(e))
}

#[derive(Clone)]
pub(crate) struct ResAccess {
    from_world: ReflectMut,
    path: Option<ParsedPath>,
}
// TODO(perf): consider storing a pointer offset and cast from `Ptr<'a, _>`
// taken from `World::get_*_by_id(ComponentId) + offset` instead of Option<ParsedPath>
#[derive(Clone)]
pub(crate) struct OneAccess {
    get: fn(&mut World) -> Result<Ref<dyn Reflect>, QuerySingleError>,
    path: Option<ParsedPath>,
}
#[derive(Clone)]
pub(crate) struct MarkedAccess {
    get_entity: fn(&mut World) -> Result<Entity, QuerySingleError>,
    from_entity: FromEntity,
    path: Option<ParsedPath>,
}
#[derive(Clone)]
pub(crate) struct NameAccess {
    entity: Entity,
    from_entity: FromEntity,
    path: Option<ParsedPath>,
    name: Name,
}
impl ResAccess {
    fn get<'a>(&self, world: &'a mut World) -> GetResult<Ref<'a, dyn Reflect>> {
        let resource = reflect_ref(self.from_world, world).ok_or(GetError::NotInWorld)?;
        read_path(&self.path, resource)
    }
    fn new(type_name: &str, path: &str, world: &World) -> NewResult<Self> {
        Ok(ResAccess {
            from_world: get_data(world, type_name, cast_to_resource_fns)?.reflect_unchecked_mut,
            path: get_path(path)?,
        })
    }
}
impl OneAccess {
    fn get<'a>(&self, world: &'a mut World) -> GetResult<Ref<'a, dyn Reflect>> {
        Ok((self.get)(world)?.map_failable(|r| read_opt_path(&self.path, r))?)
    }
    fn new(one: &str, path: &str, world: &World) -> NewResult<Self> {
        Ok(OneAccess {
            get: get_data(world, one, get_queryable_fns)?.get_single_ref,
            path: get_path(path)?,
        })
    }
}
impl MarkedAccess {
    fn get<'a>(&self, world: &'a mut World) -> GetResult<Ref<'a, dyn Reflect>> {
        use GetError::NoComponent;

        let entity = (self.get_entity)(world)?;
        // SAFETY: we just got the entity from the world.
        let entity = unsafe { world.get_entity(entity).unwrap_unchecked() };
        let entity = (self.from_entity)(entity).ok_or(NoComponent)?;
        read_path(&self.path, entity)
    }
    fn new(marker: &str, accessed: &str, path: &str, world: &World) -> NewResult<Self> {
        Ok(MarkedAccess {
            from_entity: get_data(world, accessed, get_queryable_fns)?.reflect_ref,
            get_entity: get_data(world, marker, get_queryable_fns)?.get_single_entity,
            path: get_path(path)?,
        })
    }
}
impl NameAccess {
    fn get<'a>(&self, world: &'a World) -> GetResult<Ref<'a, dyn Reflect>> {
        use GetError::{NoComponent, NoEntity};

        let entity = world.get_entity(self.entity).ok_or(NoEntity)?;
        let entity = (self.from_entity)(entity).ok_or(NoComponent)?;
        read_path(&self.path, entity)
    }
    fn new(name: String, accessed: &str, path: &str, world: &mut World) -> NewResult<Self> {
        let name: Name = name.into();
        let not_in_world = || ParseError::NotInWorld(name.to_string());
        Ok(NameAccess {
            from_entity: get_data(world, accessed, get_queryable_fns)?.reflect_ref,
            entity: get_with_name(world, &name).ok_or_else(not_in_world)?,
            path: get_path(path)?,
            name,
        })
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
