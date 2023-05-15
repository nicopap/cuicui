# Rich text

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

- [ ] Documentation
    - [ ] Instructions on how to add richtext to your project (but avoid explicitly naming version,
          or use a crate to maintain it)
    - [ ] This time, the README is different from the crate root's doc string, so we need to check
          the code listings manually
    - [ ] Important items:
        - [ ] `RichTextPlugin`
        - [ ] `RichText`
        - [ ] `RichText::parse`
        - [ ] `RichTextData`
        - [ ] `RichText::set`-family
        - [X] `modify` module
        - [X] crate root.
- [X] Define and verify a grammar for defining rich text through a string
      like rust's format macro.
- [X] Control by name the content of sections
- [X] Control styling of sections through modifiers
- [X] Control by name the modifiers at runtime
- [X] Allow nesting of sections, so that outer modifiers affect inner sections.
- [X] Check that nesting doesn't overwrite parent modifiers.
- [X] Use modifier type id for implicit references.
- [X] Implement ez methods for implicit refs
- [X] Use a `Resource` context instead of storing it in a `Component`.
- [X] Proper error handling when parsing keys/values
- [ ] Text2d support (maybe even _generic_ support)
- [X] Control a bevy `Text` by manipulating `RichTextData`
- [X] Provide systems to automatically update `Text` based on `RichTextData`
- [X] `Fetcher`s and `Tracker`s
  - [X] `Tracked`
  - [X] `DebugTracked`
  - [X] resource tracker
- [ ] Reflection handling
    - [X] `Format` cleanup as described in design_doc/richtext/dynamic_format.md
    - [ ] A `{Named(Name).Component.path.to.field}` path specifier.
    - [X] "Pull bindings" format string decides what to read rather than `tracker`s
    - [X] namespaced binding -> Require update to grammar.
    - [X] Design a reflection-based format system.
    - [X] Prepare code for pull formatting
        - [X] Separate `RichText` from datastructures used for parsing
        - [X] Custom `Modify`, registration, name, parse
- [ ] Lean on reflection for Resource modifiers
    - [ ] Allow arbitrary modifiers from `Format`
    - [ ] Allow user-defined `Format`s
    - [ ] Remove `ResourceTrackerExt`
    - [X] Consider using "starting by 'Res'" or "format applies to everything"
          in order to avoid `fmt:` prefix → Actually checking for closing
          delimiter is much easier.
- [X] Refactor
  - [X] extract richtext into separate crate
  - [X] Reorganize modules: `trackers`, `modify` (trait) `modifiers` (impls)
        `parse`, `plugin`, `change_check`
  - [X] Replace hackish implementation of `Bundle` with simple macros
  - [X] Remove dead code (existed only so that it can be stored in git history for later retrieval)
- [ ] Optimization: Early abort on resource extracted is not changed.
- [ ] Optimization: CRITICAL: fix bindng change bit not being reset after application.
- [ ] (unsure) Optimization: Consider `inplace_it` crate for some arrays.
- [ ] Optimization: compare Target values _before_ formatting them
- [ ] Optimization: do not create a string, but instead `clear` and `write!` to 
      sections.
- [ ] Way to avoid warnings when inserting the RichText
- [ ] Better error model than `anyhow`
- [X] Limit amount of updating by implementing a finer-grained change
      detection system in `RichTextData`
- [ ] Optimization: update Cow instead of creating new one => no alloc
- [ ] Extract `Modify<T>` to be generic over what it modifies
      + `Context` as associated type of `T` most likely.
    - [ ] Nested Modifiers
        - [X] Support downstream change trigger (`Modify::changes` method)
        - [X] Keep ordering of `Modify` that affect the same region
        - [ ] Remove `Dynamic` as a modifier
        - [ ] Remove `Content` as a modifier
        - [ ] Complete implementation
            - [ ] RichText::root_mask_for
            - [ ] RichText::binding_modify
            - [ ] Make::purge_static
            - [ ] Make::modify_deps
            - [ ] Make::binding_mask
    - [ ] extract `store` module into individual crate.
    - [ ] Clean up `bindings.rs`, `richtext.rs`, `richtext/make.rs`, `modify.rs`
        - [ ] A lot actually belong to `Modify`
        - [ ] It should be generic over what is being modified
        - [ ] `sort` type-safe slices for usage in `Modifiers` and `Dependencies`
              to ensure we indeed sort our stuff correctly
- [ ] (unsure) optimization: take inspiration from https://github.com/Wallacoloo/jagged_array/blob/master/src/lib.rs#L68 for `VarMatrix`s impls
- [ ] (unsure) better format string error messages
- [ ] (unsure) Allow compile-time verification of rich text spec through a
      proc macro

[misery-bui]: https://github.com/bevyengine/bevy/blob/22121e69fb4a72bb514d43240df220b8938a1e13/examples/3d/shadow_biases.rs#L107-L141
