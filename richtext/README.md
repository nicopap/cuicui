# Rich text

https://github.com/nicopap/cuicui/assets/26321040/e81b2dae-1dda-4188-ace1-6c2a8316c90c

<details>
  <summary>Click to see code</summary>
  
```rust
fn setup_system(mut commands: Commands) {
    commands.spawn((
        MakeRichTextBundle::new(
            "{Color:{color}|{Rainbow:20.0|Bonjour} {greeted}!}\n\
            {Color:Yellow, Sine:80|We are having fun here, woopy!}",
        )
    ));    // ...
    commands.spawn((
        MakeRichTextBundle::new(
            "FPS: {Font:fonts/FiraMono-Medium.ttf, Color:gold, Content:{Res.Fps.fps:.1}}",
        )
    ));    // ...
}
fn color_update_system(time: Res<Time>, mut query: Query<&mut RichTextData, With<ColorText>>) {
    for mut text in &mut query {
        let seconds = time.elapsed_seconds();
        let new_color = ;// ...;
        text.bindings
            .set("color", modifiers::Modifier::color(new_color));
    }
}
fn greet_update_system(
    mut query: Query<&mut RichTextData, With<ColorText>>,
    mut current_guest: Local<usize>,
) {
    let at_interval = |t: f64| current_time % t < delta;
    for mut text in &mut query {
        if at_interval(1.3) {
            *current_guest = (*current_guest + 1) % GUESTS.len();
            text.set_content("greeted", &GUESTS[*current_guest]);
        }
    }
}

```

</details>

A rich text component for bevy.

The current bevy `Text` component is [a misery to use][misery-bui].

`RichText` "manages" `Text` sections,
it associates section contents and styles to a "binding" (a name).
As a user, you set the value of bindings through `richtext_data.set(&str, value)`.

It's already much better than `Text`. It has some issues though:

- It's still verbose to update text: add marker component,
  query for it in a system, call the `set` method.
- you can make typos.

I don't have a solution for typos,
but I can work around it by solving the other issue.

Consider a game options menu.
What an options menu does **in a bevy game**
is usually set (and read) the values of some component or resource fields.

| 📝 [Read a shorter intro][docsrs-root] | 
|----------------------------------------|


### With cuicui_richtext

```rust
let instructions =
  "Controls:\n\
  WSAD  - forward/back/strafe left/right\n\
  E / Q - up / down\n\
  L     - switch between directional and point lights [{light_type}]\n\
  1/2   - change point light depth bias [{point_depth_bias}]\n\
  3/4   - change point light normal bias [{point_normal_bias}]\n\
  5/6   - change direction light depth bias [{directional_depth_bias}]\n\
  7/8   - change direction light normal bias [{directional_normal_bias}]\n";

let style =  TextStyle {
    font_size: 20.,
    ..default()
};
commands.spawn(RichText::parse(instructions, style).unwrap());

// ...

let light_type =  if point_light { "PointLight" } else { "DirectionalLight" };
// without macro:
example_text.set_content("light_type", light_type);
```

## Rich text

```
This line {color: blue |contains} multiple {size: 3.0, font: bold.ttf |sections}
```

| 📝 [Read the format string grammar][fs-grammar] | 
|-------------------------------------------------|

Rich text is a bevy plugin to simplify text management in bevy. It can be thought
as 3 modules:

1. A parser that reads a format string and translates it into
2. A `RichText` specification, a series of sections of text with modifiers 
3. A bevy plugin that reads that specification, and with additional context information
   manipulates a bevy `Text`.

### Jargon

A `RichText` is a series of `Section`s. Sections are specified between
curly brackets, and contain:

- _format string_: The string we parse to create a `RichText`
- _specified_: Stuff that is defined within the _format string_.
- Rust types are `MonospacedCamelCase`.
- Element of texts found in the format string, or specifying text found
  in the format string are _`monospaced_italic`_.
- _modifiers_: See section [#modifiers].
- _dynamic modifiers_: See section [#dynamic-modifiers].
- _Bindings_: the names by which _dynamic modifiers_ are referred to.
- _Type bindings_: are _dynamic modifiers_ without an explicit name, they can
  only be referred to by the type of the modifier.

### Section

A `RichText` is split in multiple _sections_, each section contains text and
additional information relative to this text.

1. Multiple _`key`_ : _`value`_ pairs, specifying _modifiers_.
2. A single text segment, specified after a _`|`_.

### Modifiers

Modifiers affect the style of the text for a given section.

The default modifiers are:

- _`Color`_: The color of text for this section, supports multiple formats:
    - html-style hex: _`#acabac`_
    - css-style function, with 3 arguments or 4 for alpha:
        - _`rgb(u8,u8,u8[,u8]?)`_ (range 0-256)
        - _`rgb(f32,f32,f32[,f32]?)`_ (range 0-1)
        - _`hsl(f32,f32,f23[,f32]?)`_ (ranges [0-360], [0-1], [0-1])
    - named constants, see the bevy [`Color`] for a list of available names
- _`Font`_: A file path in the `assets` directory. You must first load that file
  and store a `Handle<Font>` to it, otherwise it won't load automatically.
- _`RelSize`_: Size relative to the root style

```
Some text {Font:bold.ttf|that is bold} and not anymore
{RelSize:0.5|The next line spells "rainbow" in all the colors of the rainbow}
{Color:red|r}{Color:orange|a}{Color:yellow|i}{Color:green|n}{Color:blue|b}{Color:indigo|o}{Color:violet|w}
{Color: rgb(10,75, 10) | Colors can be} {Color: #ab12fa|specified in many} {Color: hsl(98.0, 0.9, 0.3)|different ways}
```

Should give (github cuts out the color, so use your imagination):

<blockquote>
<p>Some text <b>that is bold</b> and not anymore</p>
<p style="font-size:50%">The next line spells "rainbow" in all the colors of the rainbow</p>
<p><a style="color:red">r</a><a style="color:orange">a</a><a style="color:yellow">i</a><a style="color:green">n</a><a style="color:blue">b</a><a style="color:indigo">o</a><a style="color:violet">w</a></p>
<p><a style="color:rgb(10,75,10)">Colors can be</a> <a style="color:#ab12fa">specified in many</a> <a style="color:hsl(98deg,90%,30%)">different ways</a></p>
</blockquote>

### Dynamic modifiers

The previous section describes how to specify final modifier values in the format string.

To update modifier values **at runtime**, you would use a *dynamic modifier*.

Instead of specifying a value in _`value`_ position, you use _`{}`_,
you can then refer to it from your bevy app.

```
Illustration: "{Color:{}|This color is runtime-updated}"
```

```rust
let new_color: Color;
rich_text.set_typed(new_color);
```

You can also use _`{identifier}`_ to give a name to your modifier,
so you can refer to it later.

```
Illustration: "{Color:{color1}|This color}{Color:{color2}|is runtime-updated}"
```

```rust
rich_text.set("color1", new_color);
rich_text.set("color2", other_color);
```

This isn't as type-safe, but with this, you can use multiple dynamic modifiers of the same type.

### Text segment

Modifiers _always_ apply to some bit of text, therefore the text segment is
mandatory in a `Section`.

TODO: previous paragraph is patently false.

```
Some text {Color: GREEN|of the green color}.
```

The text segment of a section does actually specify the _`Content`_ modifier.
The next format string is equivalent to the previous one:

```
Some text {Color: GREEN, Content:of the green color}.
```

### Dynamic content

Similarly to other `Modify`s, you can set text content dynamically:

```
Some text {Color: GREEN, Content:{my_content}}.
```

```rust
let ammo_left = 255;
rich_text.set("my_content", modifiers::Content(format!("{ammo_left}").into()));
rich_text.set_content("my_content", ammo_left);
```

Format strings have a special syntax for content binding:

```
Some text {my_content}.
```

Finally, content can be bound by type, same as other modifiers:

```
Some text {} et voilà.
Some text {Color: GREEN, Content:{}} et voilà.
```

### Nested text segments

`RichText` is a _series_ of `Section`s. 
However, the text segment can contain itself "sub sections".

```
Some text {Color: GREEN|that is green {Font:bold.ttf|and bold {RelSize:3.0|and big}} at the same time}.
```

Subsections are flattened into a single flat list.
As expected, subsections inherit `Modify`s from their parent.

The previous format string would be split in **six** segments as follow:

```
Some text █that is green █and bold █and big█ at the same time█.
^          ^              ^         ^       ^                 ^
|          |              |         |       |                 root formatting
|          |              |         |       root + green
|          |              |         root + green + bold.ttf font + size×3
|          |              root + green + bold.ttf font
|          root + green
root formatting
```

This also works with dynamic modifiers.

It is an error to specify a `Modify` in a section and re-set it in a child section.

This doesn't work when _`Content`_ is specified as a modifier value:

```
// I've no idea what this results in, but it's definitively broken
Some text {Color: GREEN,content:that is green {Font:bold.ttf|and bold}}.
```

You can escape curly brackets with a backslash.

## Context

`cuicui_richtext`'s `RichText` component doesn't render to screen. It only is
a set of rules telling how to modify bevy's native `Text` component given a
provided context.

What is this context you are talking me about?

Bear with me. `RichText` is a list of sections, sections — as mentioned —
are a list of *modifiers* aka `Box<dyn Modify>` objects.

`Modify` is a trait:

```rust
pub trait Modify {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Result<(), AnyError>;
}
```

cuicui_richtext will run `apply` for each `Box<dyn Modify>` in a section.
But what are all those arguments? Let's see:

- `TextSection`: it's the bevy fundamental unit of text, you know it.
- `Context`: some additional info.

More precisely:

```rust
pub struct Context<'a, 'b> {
    pub registry: Option<&'b TypeRegistry>,
    pub bindings: Option<&'b Bindings>,
    pub world_bindings: Option<&'b Bindings>,
    pub type_bindings: Option<&'b TypeBindings>,
    pub parent_style: &'b TextStyle,
    pub fonts: &'a dyn Fn(&str) -> Option<Handle<Font>>,
}
```

- `parent_styles`: The base style we will dervie the style of each section
- `fonts`: Just a way to read fonts.
- `registry`: The bevy app type registry.
- `bindings`, `world_bindigns`, `type_bindings`: The interesting bit

### Bindings

Remember *dynamic modifiers*. Since they are not getting their value from the
definition of the `RichText`, they must be taking it from somewhere else. Where,
you ask? The bindings! Let's take a look at their definition:

```rust
pub type Bindings = HashMap<String, Box<dyn Modify>>;
```

It's just a map from names to `Modify`. `RichText`, instead of using a
pre-defined `Modify`, will pick it from the `Bindings` and use it.

#### Adding bindings

Currently bevy integration goes through the `RichTextData` component
and `WorldBindings` resource.

Add some rich text with the `RichTextBundle` and modify it by querying for
`RichTextData` and calling:

- `rich_text_data.set(binding_name, value)` To set a non-content binding
- `rich_text_data.set_content(binding_name, content)` to set a content binding

You can also use the similarly named methods on the `WorldBindings` resource.
`WorldBindings`, unlike `RichTextData` applies to **all** `RichText`s, not just
the one on the `RichTextData`'s entity.

With this the `RichTextPlugin` will be able to update the text sections based
on your run-time values.

### Fetchers

Fetchers let you delegate the work of reading from `World` and updating `RichText`
data to the `RichText` plugin.

They update the `GlobalRichTextBindings` based on the value of resources or components.
`cuicui_richtext` provides bundles to make this as little intrusive as possible.

```rust

fn setup(mut commands: Commands) {
    let value = 3.41;

    // If your component implements `fmt::Display`, you can use the `Tracked` bundle,
    // This will update content bound to provided name based on the value of component.
    // `track!` is a thin wrapper around `Tracked` to make it a bit less honerous to use.
    commands.spawn((
        SomeBundle { foo: 34.0, ..default() },
        track!(tracked_slider_value, Slider(value)),
    ));
    // You can use the 'd flag if you want to derive `Debug` and not have to
    // manually implement Display
    commands
        // If a bundle has the component you want to track, you should insert
        // it separately as shown here.
        .spawn(BundleWithRelevantComponent { foo: 34.0, ..default() })
        .insert(track!('d, debug_tracked_slider_value, Relevant(value)));

    // The 'm flag let you tie a value to an arbitrary modifier.
    // Your component needs to implement `IntoModify`.
    commands.spawn((
        SomeBundle { foo: 34.0, ..default() },
        track!('m, snd_line_color, UserColor(Color::PINK)),
    ));
    // You can also do this with resources. Import the `ResourceTrackerExt` trait.
    // This binds to the name of the type.
    // You can use `commands.init_tracked_resource` for default resources.
    commands.insert_tracked_resource(PlayerCount(10));

    // Works with `Modify` resources as well.
    // Those methods also exist on `App`.
    commands.insert_modify_resource(LineColor(Color::RED));

    // Rich text will automatically be updated.
    commands.spawn(RichTextBundle::parse(
        "Player count: {PlayerCount}\n\
        {Color:{snd_line_color}|slider value for name: {named_slider_value}}\n\
        slider value for entity: {entity_slider_value}\n\
        {Color:{LineColor}|slider value for from DebugTracked: {debug_tracked_slider_value}}\n\
        slider from tracked: {tracked_slider_value}",
        TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 34.0,
            color: Color::WHITE,
        },
    ));

    
}

#[derive(Component, Debug, Clone, Copy)]
struct Slider(f32);
impl fmt::Display for Slider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

#[derive(Component, Clone, Copy)]
struct UserColor(Color);
impl IntoModify for UserColor {
    fn into_modify(self) -> richtext::ModifyBox {
        Box::new(modifiers::Color(self.0))
    }
}
```

## Future work

- [`bevy_ui_bits`][bui_bits] has cool *embossed* text and preset size constants.
- It should be possible to write a macro for parsing the specification string
  at compile time
- Better API: something similar to bevy's label for the binding context, so
  that typos are caught at compile time.

## Previous work

- [**bevy_ui_bits**][bui_bits]

[bui_bits]: https://github.com/septum/bevy_ui_bits
[`fmt`]: https://doc.rust-lang.org/stable/std/fmt/index.html
[`Color`]: https://docs.rs/bevy/latest/bevy/prelude/enum.Color.html
[fs-grammar]: https://github.com/nicopap/cuicui/blob/main/design_doc/richtext/informal_grammar.md
[docsrs-root]: https://docs.rs/cuicui_richtext/latest/cuicui_richtext/

## TODO

- [ ] fab parse: performance: use jagged array for `tree::Sections` to avoid insane amount of alloc
- [ ] richtext: put the public types such as RichText & MakeRichText & WorldBindings into their own mod
- [X] all crates: Rename all occurences of "prefab"
- [ ] all crates: Do a pass on references to "text" in fab crates.
- [ ] rt_fmt: Fix formatting doing weird things
- [ ] fab_parse split: Make the API public.
- [X] fab/datazoo cleanup: remove all u32::try_from(usize) and add a const () = assert!(sizeof);
- [X] fab_derive: Write the doc strings of modify functions on the modify enum variants and constructor.
- [X] bevy_fab: Reduce the trait boilerplate.
- [ ] fab_derive: Document which fields are accessed in modify enum variant and constructor.
- [ ] fab_derive: Document `impl_modify` macro fully. Specifically: settle on a naming convention
      and use it consistently.
- [ ] fab_derive: Test `impl_modify` more thourougfully
- [ ] fab_derive: Define error messages as txt files and use `include_str` in both doc and code.
- [ ] fab: Let user specify `track::Write`s
- [ ] fab resolve: Verify validaty of multiple write fields
- [X] fab: Entry api to bindings, allows skipping allocations wholesale.
- [X] fab resolve: Implement proper dependency resolution
- [X] fab resolve: Fix modifiers overwritting static children
- [ ] fab resolve + fab_derive: Context field access tracking
- [ ] fab resolve + fab_derive: Nested fields handling, modifying (.foo → .foo.x + .foo.y)
- [X] fab resolve: Lightweight dumb resolver
- [ ] fab resolve: Test MinResolver
- [ ] richtext trackers: Cleanup error handling
- [ ] fab_parse post_process: Cleanup error handling (major issue)
- [ ] bevy_fab trackers: Manage when cached entity changes/not accessible
- [X] bevy_fab trackers: Cleanup module tree
- [ ] bevy_fab trackers: Check is_changed of resources and components before updating binding
- [ ] bevy_fab trackers: Check that the target field changed before updating binding
- [ ] bevy_fab trackers: Test the reflection-component-based trackers
- [ ] fab parse: review the informal_grammar.md file
- [ ] richtext: Text2d support
- [ ] richtext: Modify a Vec<&mut Text> over TextSections, to allow all kind of effects
- [X] richtext: way to apply the same Modify in series, by splitting text word/character
- [X] richtext split: figure out why this isn't rendered nicely.
- [X] richtext parse: Implement b2m (binding to modifier) probably with a smallvec of
      (BindingId, ModifierIndex)
- [X] richtext: Post-process content splitting as described in `post_process_content.md`
- [ ] everything: Document the hell out of everything
- [ ] bevy_fab: context-specific Resolvers. Could use a different resolver depending on the text
      being created, still sharing the same interner and Modify (though this conflicts with Resolver
      as a Modify associated type).
- [ ] fab parse: test and improve error messages
- [ ] (unsure) optimization: take inspiration from https://github.com/Wallacoloo/jagged_array/blob/master/src/lib.rs#L68 for `VarMatrix`s impls
- [ ] (unsure) richtext parser: Allow compile-time verification of rich text spec through a proc macro
- [ ] (unsure) fab resolve: Handle binding that depends on fields (Option<Modifier> in binding view)
    -> Problem: Requires clone + updating it is non-trivial

[misery-bui]: https://github.com/bevyengine/bevy/blob/22121e69fb4a72bb514d43240df220b8938a1e13/examples/3d/shadow_biases.rs#L107-L141
