# Change detection

We want finer-grained change detection, otherwise this is going to be too slow
and cause a lot of spurious uniform updates.

## What changes?

The thing that changes is the `Text` (or rather its `TextSection`s) component
neighbor of `RichText`.

## `Modify`-level change detection

Different kind of `Modify` depends on different inputs:

- `Color` and `Content` depends on **nothing**, ie: it's monotonic, calling
  `apply` more than once on the same `TextSection` is useless for those.
  Unless the things those modifiers touch were modified.
- `Font` depends on `Context.fonts`, but this _should_ never change between calls.
- `RelSize` depends on `Context.parent_style.font_size`
- `Dynamic` depends on `Context.{bindings,world_bindings,type_bindings}` and
  all the dependency of the `Modify` the binding resolves to.

## `Tracker`-level change detection

"Depends on `Context.bindings` is way too large. 1 `Dynamic` depends on 1 binding.

## `RichText`-level change detection

`RichText` is immutable. But It is that which knows `Modify`es. Hence, this is
who we ask to know what to update when something changes.

### `update`-level detection

See [./nested_modify.md]

## Interning

`RichText` now contains `dependencies`. List of `DependsOn` to `Vec<index>` where
`index` are the indices in `modifiers` of modifiers that depend on `DependsOn`.

`DependsOn` represents either a binding or any field of the `modify::Context`
struct.

We should use a string interner or `slotmap` + `HashMap<String, SlotKey>`
to convert the string into a a compact `K` inline value. This way, at runtime, we
do no comparison, and, ideally, we index by the `K`, to avoid hashing and
dereferrencing for comparision, etc.

This is incompatible in several ways with our current implementation:

- "type-bound" namespace is distinct from "name-bound" namespace
- There is a distinct "local" and "world" namespace
- Keep track of type of the binding.

### local vs world namespaces

Most notably, when building the RichText, we do not have access to the world
bindings. How are we to reconcile world and local `K`s?

Idea: `WorldBindings` should be special, and be aware of all local namespaces.
The backing storage could be `Vec<((Entity, BindingId), ModifyBox)>`.

Problem: however, we typically want the same name used in two different RichText
to be bound to the same value.

We could force to use a global interner when parsing the format string.

