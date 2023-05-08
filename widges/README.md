## Widges

Widgets are called `widges` in cuicui because I am t3h PeNgU1N oF d00m and bcuz
its SOOOO random!!!

cuicui contains N times more widgets than `bevy_ui` (and as mentioned earlier
they are also called `widges`, and _de facto_ a lot cooler)

- [ ] `Toggle`
- [ ] `Button`
- [ ] `Counter`
- [ ] `Menu`
- [ ] `Checkbox`
- [ ] `ProgressBar`
- [ ] `Slider`
- [ ] `Cancel`
- [ ] `List`

cuicui integrates [`bevy-ui-navigaiton`] and a system similar to `bevy_ui_dsl`.

## TODO

- [ ] Widges
  - [ ] ~~Prefab system~~ --> Redesign documented in design_doc/widges.md
    - [X] basic composable trait that allows spawning widgets
    - [X] composable trait to query widget value from world
  - [ ] Widge system
    - [ ] A set of simple but effective widges
      - [ ] `Toggle`
      - [ ] `Button`
      - [ ] `Counter`
      - [ ] `Menu`
      - [ ] `Checkbox`
      - [ ] `ProgressBar`
      - [ ] `Slider`
      - [ ] `Cancel`
      - [X] `List`
    - [ ] "Structural" widges based on bevy's `Reflect` trait (see `ReflectRef`)
      - [ ] `struct`
      - [ ] `enum`
      - [ ] `List`
      - [ ] `Map`
    - [ ] Gallery example.
    - [ ] System to select widges based on external definition
    - [ ] System to manipulate style-based components based on external definition
  - [ ] Windowmaker app to create re-usable widget trees.

