
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

