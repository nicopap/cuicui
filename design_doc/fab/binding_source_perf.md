# Binding source implementation

Problem: Entirely reyling on `ReflectComponent` for `Component`-based binding
sources is extremely inefficient.

Basically no way to get from a `ReflectComponent` to something in the world.

Solution: Define your own `TypeData` that stores a function taking a world
and returning list of entities with given componet.

Added bonus is that it's possible to add it to pre-existing components.