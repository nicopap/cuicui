# Global Architecture

bevy mod cuicui is a UI framework. A UI framework is surprisingly difficult 
thing to make (ok, most people are aware of this, I was just assuming they were
wrong).

A UI framework is actually many non-trivial systems deeply interwined. Which I
assume is why they are all confusing, and why it's difficult to make one.

Bevy mod cuicui has the following systems:

- A bevy-specific `Prefab` system that enables loading, reloading, querying and
  despawning parametrizeable quantums of ECS hierarchy of entity with specific
  components.
- A `Widge` (widgets) system built on prefab
- A state management system that translates a complex tree of widges into (and
  from) a more simple `struct` (or `enum`)
- A rich text component with a styling system reflecting the larger styling
  and presentation APIs.
- A presentation system that allows choosing what part of the state is
  represented with which widge
- A second, more primitive presentation system, styling, that controls individual
  fields of components of individual parts of the widges, that should integrate
  with widges so that it's possible to express styling fluently[0].
  - Of course, those two systems should be bidirectional, specifiable
    through an external asset and hot-reloadbale. This isn't about *why*, it's
    about *why not*!
- A layouting system
- An activation tree manager.


- [0] Example: A checkbox + label is an entity that represents the check, the
  box, the label. Each with many parameters. Not even taking animation in
  consideration.