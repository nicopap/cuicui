# Cuicui framework

| ❗ **THIS IS A WIP, MOST FEATURES ARE NOT IMPLEMENTED YET, SEE TODO** ❗ |
|--------------------------------------------------------------------------|

A quick and dirty UI lib for bevy built on bevy's excellent 2d primitives.

## Why?

- A very simple layout algorithm you can keep in your head.
- Reuses the same rendering as 2d, the final game binary will be smaller.
- You can directly manipulate UI element's `Transform`.
- Better integration with 3rd party crates, as it uses pre-existing 2d primitives.
  - You want to add particles? [Go ahead][bevy_hanabi].
  - You want UI element outlines? [Go ahead][bevy_2d_outline].
  - May be interested in the beta branch of [`bevy_mod_picking`]
- cuicui is built on top of a composable `Prefab` system.
- There is a few widgets I ABSOLUTELY NEED for my game, and `bevy_ui` has
  nothing more than buttons (yikes!!)
- Oh god I expected this list to only have two items

## Widges

cuicui defines a bunch of widges **NOT**.

See [`cuicui_widges`'s README for the list of innexisting widges](./widges)

## Layout

cuicui defines its own layouting algorithm.

See [`cuicui_layout`'s README](./layout).

## Rich text

cuicui defines a `RichText` component.

See [`cuicui_richtext`'s README](./richtext).

## Short term roadmap

0. [X] Fix panic on modifier parsing in richtext
0. [X] Enable usage with `Reflect` resources
1. [ ] Publish richtext
1. [ ] Implement change detection
2. [X] Study documentation, best way of presenting the crate
3. [ ] Advertise to bevy community richtext and potential for `Modify` trait
4. [ ] Abstract `Modify`, Create a `cuicui_fab` crate, dedicated to `Modify`.
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

[bevy_2d_outline]: https://lib.rs/crates/bevy_simple_2d_outline
[bevy_hanabi]: https://lib.rs/crates/bevy_hanabi
[`bevy_mod_picking`]: https://lib.rs/crates/bevy_mod_picking
[`slotmap`]: https://lib.rs/crates/slotmap
[`taffy`]: https://lib.rs/crates/taffy
[`bevy-inspector-egui`]: https://lib.rs/crates/bevy-inspector-egui
[`bevy-ui-navigation`]: https://lib.rs/crates/bevy-ui-navigation