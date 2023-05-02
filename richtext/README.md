# Rich text

Idea: It's unpleasant to work with bevy's `Text` component because of having
to individually separate sections and manually update content.

Ideally we should be using a "template string" both to specify the style and
what to put in the content, ideally update it seemlessly.

### Current

```rust
    let style = TextStyle {
        font_size: 20.,
        ..default()
    };
    commands.spawn(
        TextBundle::from_sections([
            TextSection::new("Controls:\n", style.clone()),
            TextSection::new("WSAD  - forward/back/strafe left/right\n", style.clone()),
            TextSection::new("E / Q - up / down\n", style.clone()),
            TextSection::new(
                "L     - switch between directional and point lights [",
                style.clone(),
            ),
            TextSection::new("DirectionalLight", style.clone()),
            TextSection::new("]\n", style.clone()),
            TextSection::new("1/2   - change point light depth bias [", style.clone()),
            TextSection::new("0.00", style.clone()),
            TextSection::new("]\n", style.clone()),
            TextSection::new("3/4   - change point light normal bias [", style.clone()),
            TextSection::new("0.0", style.clone()),
            TextSection::new("]\n", style.clone()),
            TextSection::new("5/6   - change direction light depth bias [", style.clone()),
            TextSection::new("0.00", style.clone()),
            TextSection::new("]\n", style.clone()),
            TextSection::new(
                "7/8   - change direction light normal bias [",
                style.clone(),
            ),
            TextSection::new("0.0", style.clone()),
            TextSection::new("]", style),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );

// ...

let new_text =  if point_light { "PointLight" } else { "DirectionalLight" };
example_text.single_mut().sections[4].value = new_text.to_string();
```

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
set_richtext_content!(example_text, light_type);
// without macro:
example_text.set_content("light_type", light_type);
```

## Rich text

```
This line {color: blue |contains} multiple {size: 3.0, font: bold.ttf |sections}
```

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

- _`color`_: The color of text for this section, supports multiple formats:
    - html-style hex: _`#acabac`_
    - css-style function, with 3 arguments or 4 for alpha:
        - _`rgb(u8,u8,u8[,u8]?)`_ (range 0-256)
        - _`rgb(f32,f32,f32[,f32]?)`_ (range 0-1)
        - _`hsl(f32,f32,f23[,f32]?)`_ (ranges [0-360], [0-1], [0-1])
    - named constants, see the bevy [`Color`] for a list of available names
- _`font`_: A file path in the `assets` directory. You must first load that file
  and store a `Handle<Font>` to it, otherwise it won't load automatically.
- _`size`_: Size relative to the root style

```
Some text {font:bold.ttf|that is bold} and not anymore
{size:0.5|The next line spells "rainbow" in all the colors of the rainbow}
{color:red|r}{color:orange|a}{color:yellow|i}{color:green|n}{color:blue|b}{color:indigo|o}{color:violet|w}
{color: rgb(10,75, 10) | Colors can be} {color: #ab12fa|specified in many} {color: hsl(98.0, 0.9, 0.3)|different ways}
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

Instead of specifying a value in _`value`_ position, you use a _`$`_,
you can then refer to it from your bevy app.

```
Illustration: "{color:$|This color is runtime-updated}"
```

```rust
let new_color: Color;
rich_text.set_typed(new_color);
```

You can also use _`$identifier`_ to give a name to your modifier,
so you can refer to it later.

```
Illustration: "{color:$color1|This color}{color:$color2|is runtime-updated}"
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
Some text {color: GREEN|of the green color}.
```

The text segment of a section does actually specify the _`content`_ modifier.
The next format string is equivalent to the previous one:

```
Some text {color: GREEN, content:of the green color}.
```

### Dynamic content

Similarly to other `Modify`s, you can set text content dynamically:

```
Some text {color: GREEN, content:$my_content}.
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
Some text {color: GREEN, content:$} et voilà.
```

### Nested text segments

`RichText` is a _series_ of `Section`s. 
However, the text segment can contain itself "sub sections".

```
Some text {color: GREEN|that is green {font:bold.ttf|and bold {size:3.0|and big}} at the same time}.
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

This doesn't work when _`content`_ is specified as a modifier value:

```
// I've no idea what this results in, but it's definitively broken
Some text {color: GREEN,content:that is green {font:bold.ttf|and bold}}.
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
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()>;
}
```

cuicui_richtext will run `apply` for each `Box<dyn Modify>` in a section.
But what are all those arguments? Let's see:

- `TextSection`: it's the bevy fundamental unit of text, you know it.
- `Context`: some additional info.

More precisely:

```rust
pub struct Context<'a, 'b> {
    pub bindings: &'b Bindings,
    pub parent_style: TextStyle,
    pub fonts: &'a Assets<Font>,
}
```

- `parent_styles`: The base style we will dervie the style of each section
- `fonts`: Just a way to read fonts.
- `bindings`: The interesting bit

### Bindings

Remember *dynamic modifiers*. Since they are not getting their value from the
definition of the `RichText`, they must be taking it from somewhere else. Where,
you ask? The bindings! Let's take a look at their definition:

```rust
pub type Bindings = HashMap<&'static str, Box<dyn Modify>>;
```

It's just a map from names to `Modify`. `RichText`, instead of using a
pre-defined `Modify`, will pick it from the `Bindings` and use it.

#### Adding bindings

Currently bevy integration goes through the `RichTextData` component.
Add some rich text with the `RichTextBundle` and modify it by querying for
`RichTextData` and calling:

- `rich_text_data.add_binding(binding_name, value)` To set a non-content binding
- `rich_text_data.add_content(binding_name, content)` to set a content binding

This is not enough to update `RichTextData`, you need to then update the bevy `Text`
component.

To do so, you can use the `RichTextSetter` world query. As follow:

```rust
fn update_text(mut query: Query<RichTextSetter, Changed<RichTextData>>, fonts: Res<Assets<Font>>) {
    for mut text in &mut query {
        text.update(&fonts);
    }
}
```

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
    commands.spawn((
        SomeBundle {
            foo: 34.0,
            ..default()
        },
        Tracked("tracked_slider_value", Slider(value)),
    ));
    // You can use `DebugTracked` if you want to derive `Debug` and not have to
    // manually implement Display
    commands.spawn((
        SomeBundle {
            foo: 34.0,
            ..default()
        },
        DebugTracked("debug_tracked_slider_value", Slider(value)),
    ));
    // `TrackedModifier` let you tie a value to an arbitrary modifier. Your component
    // needs to implement `IntoModify`.
    commands.spawn((
        SomeBundle {
            foo: 34.0,
            ..default()
        },
        TrackedModifier("snd_line_color", UserColor(Color::PINK)),
    ));


    // More fancy setups are possible with `Fetcher`s. 
    let id = commands.spawn(SliderBundle {
        slider: Slider(value),
        ..default()
    }).id();
    commands.add(AddEntityFetcher {
        entity: id,
        fetch: |s: Slider| Content::from(s.0),
        target_binding: "entity_slider_value",
    });

    // from name
    commands.spawn((
        Name::new("slider entity"),
        SliderBundle {
            slider: Slider(value),
            ..default()
        }
    ));
    commands.add(AddNamedFetcher {
        entity_name: "slider entity",
        fetch: |s: Slider| Content::from(s.0),
        target_binding: "named_slider_value",
    });

    // You can also do this with resources
    commands.insert_resource(PlayerCount(10));
    commands.add(AddResourceFetcher {
        fetch: |s: PlayerCount| Content::from(s.0),
        target_binding: "player_count",
    });

    // Rich text will automatically be updated.
    commands.spawn(RichTextBundle::parse(
        "Player count: {player_count}\n\
        {color:$snd_line_color|slider value for name: {named_slider_value}}\n\
        slider value for entity: {entity_slider_value}\n\
        slider value for from DebugTracked: {debug_tracked_slider_value}\n\
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



### Custom modifiers

`Modify` is a rust trait, sections store a `HashMap<TypeId, Box<dyn Modify>>`,
meaning that each section can have multiple types of modifiers, but at most one of each.

Since it is a public trait, users may add their own `Modify` type.
Since rich text relies on runtime reflection,
you must register your custom modifiers to be able to use them.

`Modify` has the methods `name` and `parse`.
In the section's modifiers segment the _`key`_ matches the `Modify::name`
of a registered modifier. The _`value`_ is passed to `parse`,
which returns a `Box<dyn Modify>` of itself.


## Future work

- [`bevy_ui_bits`][bui_bits] has cool *embossed* text and preset size constants.
- It should be possible to write a macro for parsing the specification string
  at compile time
- Better API: typically you'd want `Context` to be a `Res`
- Better API: something similar to bevy's label for the binding context, so
  that typos are caught at compile time.
- Better API: provide a system to automatically update the bevy `Text`.
- Provide adaptors that makes use of a (`Entity`, `ComponentId`, `ReflectPath`)
  tuple to read directly from ECS data, instead of forcing user to update
  themselves the text value.

## Previous work

- [**bevy_ui_bits**][bui_bits]

[bui_bits]: https://github.com/septum/bevy_ui_bits
[`fmt`]: https://doc.rust-lang.org/stable/std/fmt/index.html
[`Color`]: https://docs.rs/bevy/latest/bevy/prelude/enum.Color.html

## TODO

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
- [ ] Proper error handling when parsing keys/values
- [X] Control a bevy `Text` by manipulating `RichTextData`
- [X] Provide systems to automatically update `Text` based on `RichTextData`
- [ ] `Fetcher`s and `Tracker`s
  - [X] `Tracked`
  - [X] `DebugTracked`
  - [ ] `TrackReflect`
  - [X] resource tracker
  - [ ] Reflection-based resource tracker (useful for config resources)
  - [ ] (unsure) Fetch commands
- [X] Refactor
  - [X] extract richtext into separate crate
  - [X] Reorganize modules: `trackers`, `modify` (trait) `modifiers` (impls)
        `parse`, `plugin`, `change_check`
  - [X] Replace hackish implementation of `Bundle` with simple macros
  - [X] Remove dead code (existed only so that it can be stored in git history for later retrieval)
- [ ] Limit amount of updating by implementing a finer-grained change
      detection system in `RichTextData`
- [ ] Optimization: update Cow instead of creating new one => no alloc
- [ ] Custom `Modify`, registration, name, parse
- [ ] Extract `Modify<T>` to be generic over what it modifies
      + `Context` as associated type of `T` most likely.
- [ ] (unsure) better error messages
- [ ] (unsure) generalize this to widges, to create a prefab system
- [ ] (unsure) Allow compile-time verification of rich text spec through a
      proc macro
