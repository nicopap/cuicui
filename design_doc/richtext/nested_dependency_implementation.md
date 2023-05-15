# Nested Dependency implementation

So how to implement this?

## Classical graph

too much pointer chasing most likely

## ECS

I've no idea, but it sounds possible.

## Bitsets array

Assuming all `Modify` stored depend on root (or depend on M that depends on root).

- _R_ is the # of `DependsOn` except `DependsOn::Binding`, more on it later.
- _N_ is the # of `Modify`

We have a bit matrix of _R_ × _N_ bits.
Bit at row _r_, column _n_ is activated when `Modify` at index
Bit is activated when `Modify` at index _n_ DependsOn _r_.

- `Box<[ModifyBox]>`
- `BTreeMap<BindingId, SmallVec<[ModifyIndex; 2]>`


## Other

- `DependsOn` can work as an index
- Store a `dependencies: [[ModifyBox]; DependsOn#length]`
- Store a `high_order_dependencies` a struct that contains 3 fields:
  - `[u32]` mapping of `ModifyBox` index to sparse relation array.
  - Matrix `Bitset` where column is what changed and row which modify depends
    on this change
  - `[u32]` associate row index to `ModifyBox` to update

iterate through `for modify in &dependencies[depends_on]` 

## Other performance improvements

- `Dynamic` isn't a `Modify` anymore.
- `Content` row of dependencies can be removed,
  as it is always applied to a single section, never depends on anything.
