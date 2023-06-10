# Bevy Queryable `Reflect`

Bevy's `ReflectComponent` allows extracting a value from a `EntityRef` or
`EntityMut`, this can be useful, but generally not enough.

`ReflectComponent` is missing ways to _query_ for the reflected thing.
Without this ability, you would be stuck _iterating over all `EntityRef` and
checking if it contains the component in question_. This is O(n) (where `n` is
_the total count of entities_) while querying just for a single entity is O(1).

We introduce `ReflectQueryable` to fill this gap.

`ReflectQueryable` adds methods to query from a dynamic value:

- `reflect_ref`: This is similar to `ReflectComponent::reflect`, but also includes
  change tick data. This allows reading change ticks on the reflected component
  in an immutable way.
  \
  Furthermore, the `reflect_mut` method has lifetime limitations, this might be a good
  way to work around them.
- `iter{,_entities,_ref,_mut}`: Iterate over all entities with the reflected component.
  Like with `world.query`, you need to call `.iter(world)` on the return value
  to iterate over the query results.
- `get_single{,_entity,_ref,_mut}`: similar to `world.get_single`, it will return
  a value only if there is **exactly** one entity containing the reflected component.

A bit of precision:

- The `_entity` variants return an `Entity` or an iterator of entities instead
  of the `dyn Reflect` version of the component.
- The `_ref` variants return a `Ref<_>` over `_`, this let you read change information
  in an immutable maner. Note that `Ref` is **not the bevy `Ref`** but an implementation
  in `cuicui_reflect_query`. It is currently impossible to use bevy's `Ref` for this.


```rust
pub struct ReflectQueryableFns {
    pub reflect_ref: fn(EntityRef) -> Option<Ref<dyn Reflect>>,

    pub get_single: fn(&mut World) -> SingleResult<&dyn Reflect>,
    pub get_single_entity: fn(&mut World) -> SingleResult<Entity>,
    pub get_single_ref: fn(&mut World) -> SingleResult<Ref<dyn Reflect>>,
    pub get_single_mut: fn(&mut World) -> SingleResult<Mut<dyn Reflect>>,

    pub iter: fn(&mut World) -> ReflectQueryableIter,
    pub iter_entities: fn(&mut World) -> ReflectQueryableIterEntities,
    pub iter_ref: fn(&mut World) -> ReflectQueryableIterRef,
    pub iter_mut: fn(&mut World) -> ReflectQueryableIterMut,
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

Beware that **you can add them yourself** if anythingi is missing. But _please_,
if anything is missing **please open an issue**, it's hard to make sure I didn't
forget anything.

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
