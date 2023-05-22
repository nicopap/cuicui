# Templates for entities

Slowly going from exclusively working with Text to working with anything.
But a few missing keys to get the transition complete.

- How to represent non-text in format string? (heterogenous)
- How to combine together several kinds of prefabs?
- How to avoid repeating yourself?

## How to combine together several kinds of prefabs?

Currently, format string creates a **flat list** of sections.
Also, it only supports list with a single kind of items.

A prefab system would require nested layout (think menu -> menu button -> label)
Also needs heterogenous list (menu can have button, slider etc.)

Idea: Projections

### Projections

Projections are lists inside of lists. I've no idea how they could work.

Supposedly, they inherit a subset of fields of their parent, and operate on those.

### Struct as prefab

We actually don't need heterogenous list,
we could do with a "heterogenous" thing be really just a single element.

## How would it look like?

→ Let's leave this for later.

Seems like the best way of approaching it is to contribute to bevy_proto.