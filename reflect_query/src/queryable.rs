//! The [`ReflectQueryable`] and underyling [`ReflectQueryableFns`] implementations.

#[cfg(doc)]
use bevy::ecs::reflect::ReflectComponent;
use bevy::{
    ecs::{query::QuerySingleError, world::EntityRef},
    prelude::{Component, Entity, Mut, Ref as BRef, Reflect, With, World},
    reflect::FromType,
};

use crate::queries::{EntityQuerydyn, MutQuerydyn, Querydyn, RefQuerydyn};
use crate::Ref;

pub type SingleResult<T> = Result<T, QuerySingleError>;

#[rustfmt::skip]
macro_rules! docs {
    (fn $queryable_method:literal) => {
        concat!("Function pointer implementing [`ReflectQueryable::", $queryable_method, "`].")
    };
    (single $query_equivalent:literal, $output:literal, $output_link:literal) => {
        concat!(
"Get a single [`", $output, "`](", $output_link, r#") of the underyling
[`Component`] from `World`, failing if there isn't exactly one `Entity`
matching this description.

Consider using [`ReflectQueryable::"#, $query_equivalent, r#"`] followed
by `.next()` if you want to get a value even if there is more than one
`Entity` with the underlying `Component`.

# Errors

This will return an `Err` if:

 - There is no `Entity` with the underyling `Component` in `world`.
 - There is more than one `Entity` with the underyling `Component` in `world`."#
        )
    };

    (query
        $querydyn:literal, $single_equivalent:literal, $method_name:literal,
        $item:literal, $item_link:literal $(,)?
    ) => {
        concat!(
"Get a [`", $querydyn, "`] to iterate over all\n\
[`", $item, "`](", $item_link, r#") with the underlying
[`Component`] from `world`.

Use [`ReflectQueryable::"#, $single_equivalent, r#"`] for a version that returns
a single element directly.

# Example

```rust
use std::any::TypeId;
use bevy::prelude::{Reflect, ReflectComponent, Component, World};
use bevy::reflect::TypeRegistryInternal as TypeRegistry;
use cuicui_reflect_query::ReflectQueryable;

#[derive(Component, Reflect, Default)]
#[reflect(Component, Queryable)]
struct Zoobazee {
    bee: u32,
    baboo: String,
}
fn reflect_query(world: &mut World, registry: &TypeRegistry) {
    let type_data = registry
        .get_type_data::<ReflectQueryable>(TypeId::of::<Zoobazee>())
        .unwrap();

    let mut query = type_data."#, $method_name ,r#"(world);

    for element in query.iter(world) {
        println!("{element:?}");
    }
}
# fn main() {
#     let mut world = bevy::prelude::World::new();
#     let mut type_registry = TypeRegistry::new();
#     type_registry.register::<Zoobazee>();
#     reflect_query(&mut world, &type_registry);
# }
```"#
        )
    };
}
/// The function pointers used by [`ReflectQueryable`].
///
/// This is **automatically created** for you when you use:
///
/// - `#[reflect(Queryable)]` attribute with `#[derive(Reflect)]` on your type definition.
/// - `app.register_type_data::<MyType, ReflectQueryable>()`
#[derive(Clone)]
pub struct ReflectQueryableFns {
    #[doc = docs!(fn "reflect_ref")]
    pub reflect_ref: fn(EntityRef) -> Option<Ref<dyn Reflect>>,

    #[doc = docs!(fn "get_single")]
    pub get_single: fn(&mut World) -> SingleResult<&dyn Reflect>,
    #[doc = docs!(fn "get_single_entity")]
    pub get_single_entity: fn(&mut World) -> SingleResult<Entity>,
    #[doc = docs!(fn "get_single_ref")]
    pub get_single_ref: fn(&mut World) -> SingleResult<Ref<dyn Reflect>>,
    #[doc = docs!(fn "get_single_mut")]
    pub get_single_mut: fn(&mut World) -> SingleResult<Mut<dyn Reflect>>,

    #[doc = docs!(fn "query")]
    pub query: fn(&mut World) -> Querydyn,
    #[doc = docs!(fn "query_entities")]
    pub query_entities: fn(&mut World) -> EntityQuerydyn,
    #[doc = docs!(fn "query_ref")]
    pub query_ref: fn(&mut World) -> RefQuerydyn,
    #[doc = docs!(fn "query_mut")]
    pub query_mut: fn(&mut World) -> MutQuerydyn,
}

/// A [reflect trait] extending [`ReflectComponent`] with query methods.
///
/// [`ReflectComponent`] doesn't have methods to get
///
/// [reflect trait]: bevy::reflect::TypeData
#[derive(Clone)]
// allow: it would be confusing to name it different and export it as ReflectQueryable.
#[allow(clippy::module_name_repetitions)]
pub struct ReflectQueryable(ReflectQueryableFns);

impl ReflectQueryable {
    /// Return the function pointers implementing the `ReflectQueryable` methods.
    #[must_use]
    pub const fn get(&self) -> &ReflectQueryableFns {
        &self.0
    }

    /// Gets the value of this [`Component`] type from the entity as a reflected
    /// reference, with tick data in [`Ref`].
    #[must_use]
    pub fn reflect_ref<'a>(&self, entity: EntityRef<'a>) -> Option<Ref<'a, dyn Reflect>> {
        (self.0.reflect_ref)(entity)
    }
}

/// Get a single entity with the reflected queryable [`Component`].
impl ReflectQueryable {
    #[doc = docs!(single "query", "&dyn Reflect", "Reflect")]
    pub fn get_single<'a>(&self, world: &'a mut World) -> SingleResult<&'a dyn Reflect> {
        (self.0.get_single)(world)
    }
    #[doc = docs!(single "query_ref", "Ref<dyn Reflect>", "Ref")]
    pub fn get_single_ref<'a>(&self, world: &'a mut World) -> SingleResult<Ref<'a, dyn Reflect>> {
        (self.0.get_single_ref)(world)
    }
    #[doc = docs!(single "query_mut", "Mut<dyn Reflect>", "Mut")]
    pub fn get_single_mut<'a>(&self, world: &'a mut World) -> SingleResult<Mut<'a, dyn Reflect>> {
        (self.0.get_single_mut)(world)
    }
    #[doc = docs!(single "query_entities", "Entity", "Entity")]
    pub fn get_single_entity(&self, world: &mut World) -> SingleResult<Entity> {
        (self.0.get_single_entity)(world)
    }
}

/// Query all entities with the reflected queryable [`Component`].
impl ReflectQueryable {
    #[doc = docs!(query "Querydyn", "get_single", "query", "&dyn Reflect", "Reflect")]
    pub fn query(&self, world: &mut World) -> Querydyn {
        (self.0.query)(world)
    }
    #[doc = docs!(query "EntityQuerydyn", "get_single_entity", "query_entities", "Entity", "Entity")]
    pub fn query_entities(&self, world: &mut World) -> EntityQuerydyn {
        (self.0.query_entities)(world)
    }
    #[doc = docs!(query "RefQuerydyn", "get_single_ref", "query_ref", "Ref<dyn Reflect>", "Ref")]
    pub fn query_ref(&self, world: &mut World) -> RefQuerydyn {
        (self.0.query_ref)(world)
    }
    #[doc = docs!(query "MutQuerydyn", "get_single_mut", "query_mut", "Mut<dyn Reflect>", "Mut")]
    pub fn query_mut(&self, world: &mut World) -> MutQuerydyn {
        (self.0.query_mut)(world)
    }
}

impl<C: Component + Reflect> FromType<C> for ReflectQueryable {
    fn from_type() -> Self {
        ReflectQueryable(ReflectQueryableFns {
            reflect_ref: |entity| {
                let world = entity.world();
                let last_change_tick = world.last_change_tick();
                let change_tick = world.read_change_tick();
                let ticks = entity.get_change_ticks::<C>()?;

                let with_ticks = Ref {
                    value: entity.get::<C>()?,
                    is_added: ticks.is_added(last_change_tick, change_tick),
                    is_changed: ticks.is_changed(last_change_tick, change_tick),
                };
                Some(with_ticks.map(C::as_reflect))
            },
            get_single: |world| {
                let component = world.query::<&C>().get_single(world)?;
                Ok(component.as_reflect())
            },
            get_single_ref: |world| {
                let value = world.query::<BRef<C>>().get_single(world)?;
                Ok(Ref::map_from(value, C::as_reflect))
            },
            get_single_mut: |world| {
                let query = world.query::<&mut C>().get_single_mut(world);
                Ok(query?.map_unchanged(C::as_reflect_mut))
            },
            get_single_entity: |world| world.query_filtered::<Entity, With<C>>().get_single(world),
            query: |world| Querydyn(Box::new(world.query::<&C>())),
            query_mut: |world| MutQuerydyn(Box::new(world.query::<&mut C>())),
            query_ref: |world| RefQuerydyn(Box::new(world.query::<BRef<C>>())),
            query_entities: |world| {
                EntityQuerydyn(Box::new(world.query_filtered::<Entity, With<C>>()))
            },
        })
    }
}
