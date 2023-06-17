# Cuicui framework

| ❗ **THIS IS A WIP, MOST FEATURES ARE NOT IMPLEMENTED YET, SEE TODO** ❗ |
|--------------------------------------------------------------------------|

A mad experiment in making a principled UI that integrates with bevy.

## Why?

- A very simple layout algorithm you can keep in your head.
- Reuses the same rendering as 2d, the final game binary will be smaller.
- You can directly manipulate UI element's `Transform`.
- Better integration with 3rd party crates, as it uses pre-existing 2d primitives.
  - You want to add particles? [Go ahead][bevy_hanabi].
  - You want UI element outlines? [Go ahead][bevy_2d_outline].
  - May be interested in the beta branch of [`bevy_mod_picking`]
- cuicui is built on top of a composable `Modify` system, acting like prefabs/blueprints.
- There is a few widgets I ABSOLUTELY NEED for my game, and `bevy_ui` has
  nothing more than buttons (yikes!!)
- Oh god I expected this list to only have two items

## Crates

Cuicui is a collection of crates.

- `cuicui_widges`: The innexisting collection of widgets
- `cuicui_layout`: A dumb layouting algorithm you can emulate in your head.
  It provides clear error messages when you do something stupid (instead of
  itself doing something stupid)
- `cuicui_datazoo`: A collection of bit-tweedling datastructures for `cuicui_fab`.
- `cuicui_fab_derive`: the `impl_modify` macro for `cuicui_fab`
- `cuicui_fab`: A tree of modifiers that can act on a sequence of items.
  Implementing static value culling and data dependency management.
  (this is very abstract, but useful for `cuicui_richtext`)
- `cuicui_fab_parse`: A parser to generate a modifier tree from a format string
- `cuicui_reflect_query`: A bevy `Reflect` trait to query entities and `&dyn Reflect`
  directly from the world.
- `cuicui_bevy_fab`: An adapter to plug `cuicui_fab` into bevy. This defines
  not only how `Resolver` fits in bevy's ECS, but also how to hooks into the ECS
  to read values declared in the format string
- `cuicui_richtext`: A rich text component for bevy
- `cuicui_bevy_layout_offset`: A small bevy plugin to manipulate UI element transform


### History

It first started as an alternative UI library for bevy, I didn't vibe with
flexbox, so I invented my own layouting algorithm. I got this far.
Then I wanted to design a UI system based on hot reloading, inspired by the
work I did on [Klod].

I got stuck in endless design parallysis. Then I went and did other things
(including implementing parallax mapping and morph targets in bevy).

The 25th of April 2023, I started designing a rich text component for bevy.

Little did I know I would spent the next 2 months working on this at full time.

I had in mind a syntax to cleanly declare text styling and maybe eventually
generalize it to more UI components, so I decided to make it live in this
repository. "Maybe eventually generalize" is the number one cause of developper
disapearance in the XXIst century. This was a prime example of overengineering.

It quickly got out of hand. I initially "just wanted" an abstraction over the
bevy `Text` and its section so that I could set a value by name rather than
painstakingly declare each section independently then index the right one.

But feature creep creeped into the feature list.

* Now I wanted to nest sections into other ones.
* Let's reimplement this using [`nom`], wait I can't understand the errors, let's use
  [`winnow`] instead! Much better!
* What about this cool effect in paper mario where the text is rainbow? How to
  split the text in smaller sections?
* Hmm, I'd like to access and format directly values from any field in the ECS
  (yes, this is possible with `bevy_reflect`)
* Dang I love the embossed effect of [`bevy_ui_bits`](bui_bits), I need a way to
  manipulate whole text components rather than just sections.
* Wait? This look like I'm reimplementing React. Does this mean I should step back?
  Noooo, of course! Let's make it a UI library.

Well, regardless of how we ended up here, we are definitively here, and as far
as I know, there is no going back. Is it wishable? Maybe, I mean, I still don't
have a useable rich text component, and in perspective I've still a couple months
work in front of me before I do.

### Dependency tree

```mermaid
flowchart LR
  datazoo["datazoo"]
  fab_derive["fab_derive"]
  fab["fab"]
  fab_parse["fab_parse"]
  reflect_query["reflect_query"]
  bevy_fab["bevy_fab"]
  richtext["richtext"]
  bevy_layout_offset["bevy_layout_offset"]
  datazoo --> fab
  fab_derive --> fab
  fab --> fab_parse & bevy_fab
  fab_parse --> bevy_fab & richtext
  reflect_query --> bevy_fab
  bevy_fab --> richtext
  bevy_layout_offset --> |feature = "cresustext"| richtext
```

## Reflect Query

Bevy lacks a to _query_ for reflected `Component`s.
Without this ability, you would be stuck _iterating over all `EntityRef` and
checking if it contains the component in question_.

`reflect_query` defines `ReflectQueryable`, a way to query for a given component
from the world.

See [`cuicui_reflect_query`'s README](./reflect_query)


## Widges

cuicui defines a bunch of widges **NOT**.

See [`cuicui_widges`'s README for the list of innexisting widges](./widges)

## Layout

cuicui defines its own layouting algorithm.

See [`cuicui_layout`'s README](./layout).

## Fab

A Reactive programming framework with no state management.

Since we are building on bevy, there is absolutely no point in reinventing
state mangement in our UI framework. For all intent and purposes, bevy's `World`
is where the state is at.

See [`cuicui_fab`'s README](./fab).

## Rich text

cuicui defines a `RichText` component.

See [`cuicui_richtext`'s README](./richtext).

## Short term roadmap

0. [X] Fix panic on modifier parsing in richtext
0. [X] Enable usage with `Reflect` resources
1. [ ] Publish richtext
1. [X] Implement change detection
2. [X] Study documentation, best way of presenting the crate
3. [ ] Advertise to bevy community richtext and potential for `Modify` trait
4. [X] Abstract `Modify`, Create a `cuicui_fab` crate, dedicated to `Modify`.
5. [ ] Study bevy_proto, how could the `Modify` trait be integrated?
6. [ ] Go back to cuicui_layout, shortcomings, usage limitations
7. [ ] Improve cuicui_layout based on that.
8. [ ] Publish layout
9. [ ] Document cuicui_layout (same as step 2 but for cuicui_layout)
10. [ ] 2nd round of cuicui_layout advertisement in bevy community
11. [ ] Abstract cuicui_layout over storage (ie: support `slotmap`), split crate
        in two `cuicui_bevy_layout` & `cuicui_layout`.
12. [ ] Contribute a cuicui_layout adapter to taffy.

Other plans:

- Integrate `bevy-ui-navigation`.
- Integrate `bevy_mod_picking` once beta branch is mainlined.


## License

Copyright © 2023 Nicola Papale

This software is licensed under either MIT or Apache 2.0 at your leisure.
See `licenses` directory for details.

[bevy_2d_outline]: https://lib.rs/crates/bevy_simple_2d_outline
[bevy_hanabi]: https://lib.rs/crates/bevy_hanabi
[`bevy_mod_picking`]: https://lib.rs/crates/bevy_mod_picking
[`slotmap`]: https://lib.rs/crates/slotmap
[`taffy`]: https://lib.rs/crates/taffy
[`bevy-inspector-egui`]: https://lib.rs/crates/bevy-inspector-egui
[`bevy-ui-navigation`]: https://lib.rs/crates/bevy-ui-navigation
[Klod]: https://gibonus.itch.io/the-boneklod
[bui_bits]: https://github.com/septum/bevy_ui_bits
[`nom`]: https://lib.rs/crates/nom
[`winnow`]: https://lib.rs/crates/winnow
