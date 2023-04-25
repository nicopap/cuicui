# Activation Tree

Goal of `label` widge: extend the click area of another widge (such as
`checkbox` or `toggle`) to encompas not only the child widge, but the whole
child widge + `label` text.

How to do it? Consider this: We use `bevy-ui-navigation`, it relies exclusively
on `Focusable`, and how does `Focusable` knows its being selected? Through the
`NavRequest` events! That we will have total control over.

So we need some event "rail switch" system on top of the likely-mod_picking
based selection system.

## Delegation

But it does mean we need to be aware of what `Entity` we need to activate with
which screen area. (since the navigation system is `Entity` dependent)

### Is this a property of `Widge`?

A `Widge` may encompass several `Widge`, and they may also have no activations.

Seems this isn't a widge property, so how to express it?

### Activation tree

A classic solution to this is to give each widge the responsability to handle
clicks. So whenever a `Widge` is embedded into another one, they lose all
rights to handle click events, and must now be entirely dependent on their
parent to charitably transimit them clicks.

This seems fine at first glance but there is an issue: what about dragging,
hovering, basically things that happens once per frame?

This would involve descending the activation tree the whole way every frame.
Maybe that's alright? It's like a scene graph, it should enable logarithmic
traversal unlike the current solution which involves iterating over all
entities that have a selectable area.

Regarding dragging: we also want to disable all other forms of picking when
dragging. It feels an activation tree is the wrong abstraction, if you need to
add to it many knobs in order to make it useable.

**But responsablity is burden**. ie: a footgun. I build a container widge and
now "oh crap! I need to do all those things!" (or forget, and things may or
may not work, and I would be at a loss at why things are not working).
Supposedly the computer is smarter than us at kind of things.

=> Good default beahvior? => How would that look like?

#### Bottom-up activation tree?

Those are issues if parents get all the responsability. What about holocracy?
This allows not carring about activation when you are not working on activation.

=> For this to work, you need to make it explicit that you need to inform
others that you rely on activations, so that widges that manipulate inner
widge's activation zone always work.

### How does this look like as an API?

Ideally no special precaution to make when creating a widge.

- Assume using `bevy-ui-navigation`
- Assume using `bevy_sprite`

It should be natural to define the `Widge`, ideally, the `Focusable` component
is added by `Prefab::spawn`.

If `Focusable` is a widge, then it can handle handle itself maintaining the
relationship.