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
update_richtext!(example_text, light_type);
// without macro:
example_text.update(hash_map!{"light_type": light_type});
```

## Default Syntax

Since all fields of `RichText` is public, and that `StyleMod` is a trait, making
it possible to extend with your own style modifiers, it's possible to write our
own parsers (for example, say, to parse markdown).

However, `RichText` is deliberately limited. It only supports a syntax derived
from rust's [`fmt`] syntax.

- A `RichText` has a root style `TextStyle` and a series of *sections*.
- A *section* is a list of *content modifiers*.
- *modifiers* are either predefined static values or dynamic runtime values.

### Sections

In term of syntax, a *section* is a comma-separated list of `key:value` pairs
within brackets.

```
// A section
{color:blue,font:fonts/fira-sans.ttf,size:0.3,content:this is a section of text}
```

`RichText` supports 4 modifiers:

- `color`: modify color of the text, default is white, on the value side, you
  can either use a color name or rgb(123,34,334) etc.
- `font`: the full path name of a loaded font.
- `size`: Size of this section relative to the size of the `RichText`
- `content`: The text content of this section

The content is *inside* the curly braces, see the difference
between **wrong**: `{color:blue}some content{/}` and
**correct**: `{color:blue,content:some content}`.

### Plain text

More simply, you can elide the `content` at the very end `{color:blue,some content}`,
the last section element is assumed to be the text content.

If there is no style modifiers, you can omit the curly braces:

```
// All of those are equivalent
{color:white,content:some text}
{content:some text}
{color:white,some text}
some text
// Note that this IS NOT equivalent
{some text}
```

### Dynamic modifiers

Similarly to rust's `println!` macro, you can create *references* to style and
text content by suffixing the `value` side of the `key:value` pair with a `$`
dollar sign.

```
{color:custom_color$,content:special_content$}
```

Those are *dynamic modifiers*. You can update them using `RichText::update`
method, by passing a `map: HashMap<String, Box<dyn Any>>` as argument
 — the actual signature of `update` is different, but equivalent,
we simplify here for clarity.

The keys of `map` are the *reference* names (without the $ sign) and the values
are what to put at the place of the reference.

In the previous example, we would use the map as follow:

```rust
let mut map = HashMap::default();
map.insert("custom_color", Color::ORANGE);
map.insert("content", "Some content".to_owned());
rich_text.update(&map);
```

Remember how `{some text}` is not equivalent to `some text`? This is because
`{some_text}` is shorthand for `{content:some_text$}`

This doens't support **bold** or *italic*, just named fonts. Neither does
it support nesting multiple levels of style, only a single level.

And yeah, you can't even have fonts with commas in their file name (you cursed soul).

As `cuicui_richtext` evolves, this might change to encompasses more use cases.
However, to get something out the window, something to start talking about,
I thought it necessary to start with this very limited API.

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


## Future work

- [`bevy_ui_bits`][bui_bits] has cool *embossed* text and preset size constants.
- It should be possible to write a macro for parsing the specification string
  at compile time
- Use `nom` for parsing, the recursive descent is cool but difficult to maintain
  and justify.
- Better API: typically you'd want `Context` to be a `Res`
- Better API: something similar to bevy's label for the binding context, so
  that typos are caught at compile time.
- Better API: provide a system to automatically update the bevy `Text`.

## Previous work

- [**bevy_ui_bits**][bui_bits]

[bui_bits]: https://github.com/septum/bevy_ui_bits
[`fmt`]: https://doc.rust-lang.org/stable/std/fmt/index.html