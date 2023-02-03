# Beyv Cuicui

A quick and dirty UI lib for bevy built on bevy's excellent 2d primitives.

## Why?

- A very simple layout algorithm you can keep in your head.
- Reuses the same rendering as 2d, the final game binary will be smaller.
- `bevy_ui` is so barebone that really it isn't difficult to reach feature
  parity. Especially if you've bee using [`bevy_mod_picking`].
- Due to <https://github.com/bevyengine/bevy/issues/5721> it's not possible
  to display something on top of UI in bevy
- Bevy's UI is basically just squares, using sprites allows getting
  beyond that. Specifically, you can directly manipulate the element's
  `Transform`
- Better integration with 3rd party crates.
  You want to add particles? [Go ahead][bevy_hanabi].
  You want sprite outlines? [Go ahead][bevy_2d_outline].
- There is a few widgets I ABSOLUTELY NEED for my game, and `bevy_ui` has
  nothing more than buttons (yikes!!)
- Oh god I expected this list to only have two items

## Widgets

Widgets are called `widges` in cuicui because I am t3h PeNgU1N oF d00m and bcuz
its SOOOO random!!!

cuicui contains N times more widgets than `bevy_ui` (and as mentioned earlier
they are also called `widges`, and _de facto_ a lot cooler)

- `ActionButton`
- `Counter`
- `Menu`
- `Checkbox`
- `ProgressBar`
- `Cancel`
- `List`

cuicui integrates [`bevy-ui-navigaiton`] and a system similar to `bevy_ui_dsl`.

## Layout

cuicui defines its own layout algorithm.

### Why not Flexbox

You are writing text to get 2d visual results on screen.
The translation from text to screen should be trivial, easy to do in your head.
Otherwise you need visual feedback to get what you want.
Bevy, even with hot reloading or [`bevy-inspector-egui`]
will always have extremely slow visual feedback.

Flexbox has too many parameters and depends on implicit properties of UI elements,
it is not possible to emulate it in your head.

cuicui's layout in contrast to Flexbox is easy to fit in your head.
In fact, I will forecefully push cuicui's layout algorithm in your head
in two short bullet points.

- A node has a size
- A node can be a `Node::Container` and distribute its children
  along a `Direction` either by evenly spacing them (`Stretched`)
  or putting them directly one after another (`Compact`).

That's it. There are some edge cases, but cuicui will ~~yell at you~~
tell you nicely when you hit them and tell you how to handle them properly.

### Why cuicui layout

On top of the very friendly layout algorithm,
cuicui runs on `bevy_ecs` and therefore can ~~abuse~~ use it as a backing storage.

Layouts are generally backed by a tree,
[`taffy`]'s implementation of Flexbox internally uses a [`slotmap`].
cuicui uses the ECS, which is basically a slotmap.

Also, cuicui's layouting system relinquishes control to give more power to users.
Meaning that you can tell cuicui to not manage UI entities `Transform`
and instead chose yourself to build the UI based on what info cuicui gives you.

## Usage

The `CuicuiPlugin` by default manages the `Transform` of spawned elements.
For various reasons, you may want to manage the positions yourself and use
the `Components` managed by cuicui as layout suggestions instead.
In which case, add `CuicuiPlugin` the following way:

```rust
todo!()
```

[bevy_2d_outline]: https://lib.rs/crates/bevy_simple_2d_outline
[bevy_hanabi]: https://lib.rs/crates/bevy_hanabi
[`bevy_mod_picking`]: https://lib.rs/crates/bevy_mod_picking
[`slotmap`]: https://lib.rs/crates/slotmap
[`taffy`]: https://lib.rs/crates/taffy
[`bevy-inspector-egui`]: https://lib.rs/crates/bevy-inspector-egui
[`bevy-ui-navigation`]: https://lib.rs/crates/bevy-ui-navigation