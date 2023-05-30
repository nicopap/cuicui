# Binding to Modify dependency

We want to unify the binding list and the modify list in `Resolver`.
This would allow resolving binding deps exactly like a modify dep.

## Problems

- When declared, a `Binding` doesn't have a serialized value.
  This means, I can't use it to build the dependency graph.
  - However, I do have the `Modify` type as the name,
    it should technically be possible to get it.
    But requires fiddling with the `impl_modify` macro most likely.
  - This fiddling would involve creating associated const on the `Modifier`
    with the name of the modify function + `_{changes,depends}`

## Alternatives

There are two types of bindings.

1. binding without parent or child dependencies
2. binding with either parent of child dependency

Current design only allows (1), preventing cool stuff like moving rainbows.

Proposed design allows both, but would make (1) much less performant
(eg: need to clone the `Modify` inline the `Resolver`)

```rust
struct Resolver {
  modifiers: Box<[Modifier<P>]>,
  // ...
}
enum Modifier<P: Prefab> {
  Binding {
    inner: Option<P::Modify>,
    range: Range<u32>,
  },
  Modifier {
    inner: P::Modify,
    range: Range<u32>,
  },
}
```

Ok, this seems redundant, what about just modifying the existing `Modifier`?

```rust
struct Modifier<P: Prefab> {
    inner: Option<P::Modify>,
    range: Range<u32>,
}
```

Bindings without deps do not need to be inserted, `inner` stays `None`.
Bindings with deps can be inserted.
No need to store in `Resolver` whether a binding has dependencies or not,
I can call `.depends` on the `Modify` stored in the `binding::View`.

This doesn't trim the `modifiers: Box<[_]>` as much as if we removed binding modifiers.
`modifiers` might be sparse/slightly sparse.
But it doesn't matter, because we never iterate `modifiers`, and nothing depends
directly on its size.