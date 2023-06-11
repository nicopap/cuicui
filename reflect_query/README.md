# Bevy Queryable `Reflect`

Bevy's `ReflectComponent` allows extracting a value from a `EntityRef` or
`EntityMut`, this can be useful, but generally not enough.

`ReflectComponent` is missing ways to _query_ for the reflected thing.
Without this ability, you would be stuck _iterating over all `EntityRef` and
checking if it contains the component in question_. This is O(n) (where `n` is
_the total count of entities_) while querying just for a single entity is O(1).

We introduce `ReflectQueryable` to fill this gap.

## Should I use this crate?

In short, if you are asking this question, you most likely shouldn't!

If the second paragraph of this README means anything to you, as in: this is
not complete technobable, and you are like "Wow! It's possible!" then yeah babe,
that's for you!

## Usage

```rust
use std::any::TypeId;
use bevy::prelude::{Reflect, ReflectComponent, Component, World};
use bevy::reflect::TypeRegistryInternal as TypeRegistry;
use cuicui_reflect_query::{ReflectQueryable, Ref};

#[derive(Debug, Clone, PartialEq, Component, Reflect, Default)]
#[reflect(Component, Queryable)]
struct Zoobazee {
    bee: u32,
    baboo: String,
}

fn reflect_query<'w>(world: &'w mut World, registry: &TypeRegistry) -> Ref<'w, dyn Reflect> {
    let type_data = registry
        .get_type_data::<ReflectQueryable>(TypeId::of::<Zoobazee>())
        .unwrap();

    let mut query = type_data.query(world);
    for element in query.iter(world) {
        println!("{element:?}");
    }
    type_data.get_single_ref(world).unwrap()
}
fn main() {
    let mut world = bevy::prelude::World::new();
    let mut type_registry = TypeRegistry::new();

    type_registry.register::<Zoobazee>();

    let component = Zoobazee {
      bee: 32,
      baboo: "zoobalong".to_string(),
    };

    world.spawn(component.clone());

    let single_result = reflect_query(&mut world, &type_registry);
    assert_eq!(single_result.downcast_ref(), Some(&component));
}
```

## Details

`ReflectQueryable` adds methods to query from a dynamic value:

- `reflect_ref`: This is similar to `ReflectComponent::reflect`, but also includes
  change tick data. This allows reading change ticks on the reflected component
  in an immutable way.
  \
  Furthermore, the `reflect_mut` method has lifetime limitations, this might be a good
  way to work around them.
- `query{,_entities,_ref,_mut}`: Iterate over all entities with the reflected component.
  Like with `world.query`, you need to call `.query(world)` on the return value
  to iterate over the query results.
- `get_single{,_entity,_ref,_mut}`: similar to `world.get_single`, it will return
  a value only if there is **exactly** one entity containing the reflected component.

A bit of precision:

- The `_entity` variants return an `Entity` or an iterator of entities instead
  of the `dyn Reflect` version of the component.
- The `_ref` variants return a `Ref<_>` over `_`, this let you read change information
  in an immutable maner. Note that `Ref` is **not the bevy `Ref`** but an implementation
  in `cuicui_reflect_query`. It is currently impossible to use bevy's `Ref` for this.


```rust,ignore
pub struct ReflectQueryableFns {
    pub reflect_ref: fn(EntityRef) -> Option<Ref<dyn Reflect>>,

    pub get_single: fn(&mut World) -> SingleResult<&dyn Reflect>,
    pub get_single_entity: fn(&mut World) -> SingleResult<Entity>,
    pub get_single_ref: fn(&mut World) -> SingleResult<Ref<dyn Reflect>>,
    pub get_single_mut: fn(&mut World) -> SingleResult<Mut<dyn Reflect>>,

    pub query: fn(&mut World) -> Querydyn,
    pub query_entities: fn(&mut World) -> EntityQuerydyn,
    pub query_ref: fn(&mut World) -> RefQuerydyn,
    pub query_mut: fn(&mut World) -> MutQuerydyn,
}
```

### Implementations for base bevy components

Since this is not part of bevy, we need to add those to the bevy components.

Users of this lib might only care to use a subset of the
bevy crates, so we can't bulk-add our components.

This crate exposes one feature per bevy features, they are off by default, you
must explicitly enable them to register `ReflectQueryable` for bevy components:

- `register_core_pipeline`
- `register_pbr`
- `register_sprite`
- `register_render`
- `register_ui`
- `register_text`

Beware that **you can add them yourself**. But _please_, if anything is missing
**open an issue**, it's hard to make sure I didn't forget anything.

### Implementing for your own types

Like with `ReflectComponent`, you need to register the trait information
for your own types. It will look like this:

```diff
+ use cuicui_reflect_query::ReflectQueryable;

  #[derive(Reflect, Component, Default)]
- #[reflect(Component)]
+ #[reflect(Component, Queryable)]
  struct Zoobaroo {
    bidoo: u32,
    bubble: String,
    padiwoop: AlphaMode,
  }
```

Make sure to `app.register_type::<Zoobaroo>()`! and you should be good to go.

## License

Copyright © 2023 Nicola Papale

This software is licensed under either MIT or Apache 2.0 at your leisure.
See `licenses` directory at the root of the [`cuicui`](https://github.com/nicopap/cuicui)
repository for details.
