use std::ptr::NonNull;
use std::{any, fmt};

use bevy::app::AppTypeRegistry;
use bevy::core::Name;
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::ecs::world::EntityRef;
use bevy::ecs::{prelude::*, query::QuerySingleError, reflect::ReflectResourceFns};
use bevy::reflect::{ParsedPath, Reflect, ReflectPathError, TypeData};
use fab_parse::tree as parse;
use reflect_query::queries::{EntityQuerydyn, RefQuerydyn};
use reflect_query::{Ref, ReflectQueryable, ReflectQueryableFns};
use thiserror::Error;

pub type NewResult<T> = Result<T, ParseError>;
pub type GetResult<'a> = Result<Ref<'a, dyn Reflect>, GetError>;

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

/// Returned by [`Read::query`] and used by [`Read::get`].
///
/// The return value of [`Read::get`] has a lifetime bound to `World`. If we
/// read it from a `&mut` we can't use the world and the return value at the
/// same time.
///
/// It is possible to avoid this by splitting the creation of the query and
/// the consumption in two.
pub struct QueryState(QueryStateInner);
enum QueryStateInner {
    Res,
    Name,
    Marked(EntityQuerydyn),
    One(RefQuerydyn),
}
impl From<QueryStateInner> for QueryState {
    fn from(value: QueryStateInner) -> Self {
        QueryState(value)
    }
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
    /// Prepare queries so that they can be ran.
    pub fn query(&mut self, world: &mut World) -> QueryState {
        match &self.query {
            Query::Res(_) => QueryStateInner::Res.into(),
            Query::Name(_) => QueryStateInner::Name.into(),
            Query::One(access) => QueryStateInner::One(access.query(world)).into(),
            Query::Marked(access) => QueryStateInner::Marked(access.query(world)).into(),
        }
    }
    pub fn get<'a>(&mut self, state: QueryState, world: &'a World) -> GetResult<'a> {
        use QueryStateInner::*;
        match (&self.query, state.0) {
            (Query::Res(access), Res) => access.get(world),
            (Query::One(access), One(state)) => access.get(state, world),
            (Query::Name(access), Name) => access.get(world),
            (Query::Marked(access), Marked(state)) => access.get(state, world),
            _ => panic!("cuicui bug, shouldn't call Read::get with a query not created by it"),
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
pub fn reflect_ref(f: ReflectMut, world: &World) -> Option<Ref<dyn Reflect>> {
    // SAFETY: We convert this immediately into immutable value
    let reflect_mut = unsafe { f(world.as_unsafe_world_cell_readonly()) };
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
fn read_path<'a>(path: &Option<ParsedPath>, reflect: Ref<'a, dyn Reflect>) -> GetResult<'a> {
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
    get: fn(&mut World) -> RefQuerydyn,
    path: Option<ParsedPath>,
}
#[derive(Clone)]
pub(crate) struct MarkedAccess {
    get_entity: fn(&mut World) -> EntityQuerydyn,
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
    fn get<'a>(&self, world: &'a World) -> GetResult<'a> {
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
    fn query(&self, world: &mut World) -> RefQuerydyn {
        (self.get)(world)
    }
    fn get<'a>(&self, mut state: RefQuerydyn, world: &'a World) -> GetResult<'a> {
        let component = state.get_single(world)?;
        Ok(component.map_failable(|r| read_opt_path(&self.path, r))?)
    }
    fn new(one: &str, path: &str, world: &World) -> NewResult<Self> {
        Ok(OneAccess {
            get: get_data(world, one, get_queryable_fns)?.query_ref,
            path: get_path(path)?,
        })
    }
}
impl MarkedAccess {
    fn query(&self, world: &mut World) -> EntityQuerydyn {
        (self.get_entity)(world)
    }
    fn get<'a>(&self, mut state: EntityQuerydyn, world: &'a World) -> GetResult<'a> {
        let entity = state.get_single(world)?;
        // SAFETY: we just got the entity from the world.
        let entity = unsafe { world.get_entity(entity).unwrap_unchecked() };
        let entity = (self.from_entity)(entity).ok_or(GetError::NoComponent)?;
        read_path(&self.path, entity)
    }
    fn new(marker: &str, accessed: &str, path: &str, world: &World) -> NewResult<Self> {
        Ok(MarkedAccess {
            from_entity: get_data(world, accessed, get_queryable_fns)?.reflect_ref,
            get_entity: get_data(world, marker, get_queryable_fns)?.query_entities,
            path: get_path(path)?,
        })
    }
}
impl NameAccess {
    fn get<'a>(&self, world: &'a World) -> GetResult<'a> {
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
