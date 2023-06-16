# Hierarchy-scoped queries

So here is another crazy idea. Relations could make it trivial to implement.

This is something I thought of during bevy jam 2, when I was making the `Prefab`
system for the game editor.

This would also allow me to replace `impl_modify` macro by the world query
systems.

I'd like a way to run queries on only on a subset of all entities. Typically,
I'd want to run a query iterating over components on entities children
(recursively) of given entity.

It's actually possible to implement already, even as a 3rd party crate,
by manually implementing `SystemParam` on — say ­— `ScopedQuery`.
The `ScopedQuery`'s `iter` and `get` methods would require a `Entity` parameter.
A naive implementation would internally use `iter_many{_mut}` internally.

A more interesting version of this — not sure if possible today, in bevy or
as 3rd party crate — could detect multiple `ScopedQuery` that run on disjoint
subsets and run systems with `ScopedQuery` in parallel,
even if they mutate the same components.

This advanced version would need to rely on a component that exists
in the ECS rather than an `Entity` provided as argument within the system.

It's a bit like running a query on the `World` within a `Scene`,
but after spawning them.