# Beyv Cuicui

A quick and dirty UI lib for bevy built on bevy's excellent 2d primitives.

## Why?

- Flexbox (the default layout for bevy) has way too many parameters, and to
  get anything working, you always have to fiddle a lot. Cuicui defines its
  own extremely dumb layout algorithm, with only a couple parameters.
  Definitively not all-purpose and definitively not as polished as flexbox,
  but you can keep it in your head and even visualize the result when writting
  down the code.
- Reuses the same rendering as 2d, the final game binary will be smaller.
- `bevy_ui` is so barebone that really it isn't difficult to reach feature
  parity. Especially if you've bee using `bevy_mod_picking`.
- Due to https://github.com/bevyengine/bevy/issues/5721 it's not possible
  to display something on top of UI in bevy
- Bevy's UI is basically just squares, using sprites allows getting
  beyond that. Specifically, you can directly manipulate the element's
  `Transform`
- Better integration with 3rd party crates. You want to add particles? YES
  You want sprite outlines? YES.
- There is a few widgets I ABSOLUTELY NEED for my game, and `bevy_ui` has
  nothing more than buttons (yikes!!)
- Oh god I expected this list to only have two items

## Widgets

Widgets are called `widges` in cuicui because I am t3h PeNgU1N oF d00m and bcuz
its SOOOO random!!!

cuicui contains N times more widgets than `bevy_ui` (and as mentioned earlier
they are also called `widges`, and de facto a lot cooler)

- `ActionButton`
- `Counter`
- `Menu`
- `Checkbox`
- `ProgressBar`
- `Cancel`

cuicui integrates `bevy-ui-navigaiton` and a system similar to `bevy_ui_dsl`.