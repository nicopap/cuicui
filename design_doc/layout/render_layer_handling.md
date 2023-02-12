# Handling of RenderLayers

The UI is constructed around RenderLayers using the 2d Rendering pipeline,
the user specifies the RenderLayer to use, the library will comply

## Propagate ourselves

RenderLayers only work for individual entities, it doesn't propagate down.
This might surprise end users who just casually add a UI tree without
much more consideration than it being a UI tree.

Should we propagate it ourselves?

We could force this by pushing usage of `RootBundle` rather than `Root`,
this way, the user is made aware of it.

But is this enough? Surely not, I thinik it will be necessary to propagate
the RenderLayers.

This is more difficult than at first sight, because we want to limit the
propagation to children (recursively) of `Root`. When an entity is added,
how do we know it is a child of root? Should we iterate over the whole
tree every time an entity is added, just because it could be a recursive
child of `Root`? This seems very costly!

Could we do it like the bevy `Transform` propagation and ignore all entities
that do not have a relevant componet? Like force adding a component?
=> But then it defeats the purpose of doing it in the first place

## Decision

I think we should do nothing.

The reason being we define ourself most of the widgets, in which we can
sneak in our RenderLayers component, and if someone wants to create their own
widget, they'll have to learn how to do it, which is actually not trivial.

We can expose them to the RenderLayers quirk this way.
