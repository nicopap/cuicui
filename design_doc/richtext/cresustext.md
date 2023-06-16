# Cresus text design

Cresus text is rich text that works on entities instead of mere text sections.

The items are not element of a `Vec` but `Query` items. 

This can't just be plugged on top of existing `Modifier` trait for a few reasons:

1. `Modifier::Item` can't be `Clone`, `Send` or `Sync` anymore. Why are those trait
   necessary currently?
  - In `resolve/make.rs`, to create the initial sections.
  - This could be replaced by a `MakeItem`?
2. By virtue of being query items, `Item` must admit a lifetime.
  - Currently needs to be `'static` due to storing default value in `MakeRichtext`
    component. This is also why `Send` and `Sync`

## Goal

Have `Items` be:

```rust
struct Param<'w, 's, Ctx: SystemParam, It: WorldQuery> {
  context: Ctx,
  query: Query<'w, 's, It>
}
impl<'w, 's, Ctx: SystemParam, It: WorldQuery> Param<'w, 's, Ctx, It> {
  fn split<'a>(&'a mut self, children: &'a Children) -> (Items<'a, 'w, 's, It>, Ctx<'a>) {
    let Self { context, query } = self;
    (Items { children, query }, context)
  }
}
struct Items<'a, 'w, 's, It: WorldQuery> {
  children: &'a Children,
  query: &'a mut Query<'w, 's, It>
}
impl<'a, 'w, 's, It: WorldQuery, M> Indexed<M> for Items<'a, 'w, 's, It> 
where M: for<'b> Modify<Item<'b> = It::Item<'b>>
{
  fn get_mut<'b>(&'b mut self, index: usize) -> Option<M::Item<'b>> {
    let entity = self.children.get(index)?;
    query.get_mut(entity).ok()
  }
}
impl Modify {
  type Items = Items<'a, 'w, 's, QueryItem<?>>;
  type Item: QueryItem<
}
```

Have `Item: WorldQuery`.

What if

```rust
trait Indexed<M> {
  fn change(&mut self, index: usize, f: impl FnOnce(&mut M::Item))
}
```

## What data?

- `M::Items`:
  - `Children` comp
  - `Query<M::Item>`: Get individual items

## What am I struggling with?

It's 

- turning `MakeItem` into `Item`
- Passing the QueryItem to `apply`

`MakeItem` 2 `Item` hard because we have two representations

- Owned (that can be stored and used as "root" when root updated)
- By ref (that can be queryed from the world)

The QueryItem is a `(Mut<X>, &Y, Option<Mut<Z>>)` while the owned thing is
`(X, Y, Option<Z>)`.

Is owned needed?

- We use it to initialize the format string, it's pretty fundamental.
- We use `apply` on this, which requires a `Item`, but since `MakeItem` is
  not `Item`, we try to work around this with `MakeItem: AsMut<Item>`, but
  it's still a problem, since ref version in `WorldQuery` is not a `&mut`, so
  we work around this by saying `&'a mut MakeItem: Into<Item<'a>>`, but then the
  trait bounds are so ridiculous it's impratical, it's also fairly error prone,
  since the goal is to store the mutable references into the target item, so that
  it's possible to update them transparently.
- Also for root updates

So we need a `MakeItem` that:

- Can be written to _as_ a `Item` (so that we can `apply` on it)
- Can take an `Item` and update its value.

Why not have a `make_default(item: &mut Item)`? Because the default changes per
instances.

Could be a method on a `MakeItem` trait?

## This is ridiculous!

New difficulty: Trying to get the old-style system working with the new one.

I'm getting close to getting something working, but it basically looks like a
giant hack that only exists to make `update_items_system` work with two completey
different things.

- `Richtext`: wants `Modify::Items` to be a component (`Text`), and `Modify::Item` an element
  of this component (`TextSection`).
  `update_items_system` iterates over all entities with a
  `LocalBindings` and `Modify::Items` component, and updates `Modify::Items` based
  on `LocalBindings`
- `Cresustext`: `Modify::Items` is a `Query` + `WorldQuery` (`&Children`),
  `Modify::Item` is return item of the `Query` in `Modify::Items` for each child.

System in question that causes soooo much trouble is:

```rust
let (context, mut items) = BM::context(params.into_inner());
for (mut local_data, wq_item) in &mut query {
  BM::set_local_items(&mut items, wq_item);
  local_data.update(&mut items, &world_bindings, &context);
}
world_bindings.bindings.reset_changes();
```

You notice `BM::context` and `BM::set_local_items` only exist for this system.
If we left definition of the system to implementor, we don't need to infect
the trait with so much nonsense.

In fact, this is also true of types `BM::Wq` and `BM::Param`!

Can't define system as an associated method or constant of `BevyModify`.
We _could_ just leave the responsability of adding an update system to the
end-user. But that seems error-prone.

Instead, we'll add a required method called `add_update_system`. Which will
force the implementor to be careful to add the method in question.

We'll provide specialized systems, so that the user can directly use them
in `add_update_system`.

