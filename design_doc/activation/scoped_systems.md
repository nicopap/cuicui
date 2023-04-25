# Scoped systems

Idea: Widges based on `Prefab` is meaningless. `Component`s are what matters,
with regard to layout, styling, visuals, etc.

What we are missing in the ECS is a way to limit system to a subset of the world.
A vocabulary to express specific interactions between entities.

## Implementation

- `Activated` component
- A registry that associate `Entity` -> `System`
- Something similar to QT signals: a way to express interaction sources,
  capture and "slots" for updating data.

System that walks downard `Entity` hierarchy with a `Activated(true)` component.
Some entities have an associated `ActiveSystem`, it is "triggered" when the
`Activated(true)` for given entity.

An `ActiveSystem` has `In<>` parameters, the walking system, when encoutering
an `ActiveSystem`, walks down the hierarchy to find entities with corresponding
`Out<>`, collect them and trigger the system.
