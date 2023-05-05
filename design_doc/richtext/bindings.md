# Binding Contexts

A `RichText` has multiple sources for context:

- `RichTextData`, a component neighbor to `RichText` that can be queried, it
  allows:
  - *by-name* bindings: you give the name in the format string, and use the
    `set("name", …)` method on `RichTextData`.
  - *by-type* bindings: you elide the name (with `$` or `{}`) and you use the
    `set_typed` method on `RichTextData`. Since `RichText` associates binding
    with a unique `Modify`, it can read the value for the associated type.
- `WorldContext` a resource.
  - By manipulating it directly with one of its mehtods.
  - Through the `track` API, which associates a component or resource to a
    binding name


## Pull vs push API

With the `track` API, we force the user to declare at to different locations
what they want to read:

- In the format string, with the `{binding_name}` or `$binding_name`
- When adding the component/resource in question, using `track!` or
  `insert_tracked_resource`.

Typically, for resource, we refer in the format string **by the name of the
resource type**, which is why we require the resource to derive `Reflect`.

For example, for a resource `struct Foobar(u32)` we will need to declare the
binding in the format string as `our foobar: {Foobar}`.

So we already know the name of the type we want to extract. Isn't that enough?
Shouldn't cuicui_richtext be able to read that and use it to _pull_ the value
of the resource, rather than waiting that we _push_ it into it?

It probably can, with `TypeRegistry::get_with_name`.

## Pull API

Let's start with pulling data from resources that derive `ReflectResource`.

My first though is: "Ok, but how do I specify how things should be displayed?"

The fields are likely to be `f32`, `Color`, etc… user likely want to **control
number of numbers after the comma**, or have show color as a color.

Those are not `Modify` or `IntoModify`, neither can user define a trait we
provide for `f32`, anyway different `f32`s should be displayed differently.

**Question 1**: How to define formatting based on resource field, rather than
field type?

**Question 2**: Once I have the `&dyn Reflect` for field, how do I print it?

### Previous work

Let's take a look at [`bevy-inspector-egui`].

They define:

- `InspectorOptions` struct
- A derive macro to generate impl of `FromType<Self> for InspectorOptions`
- `ReflectInspectorOptions` to be stored in type registry.

**Question 3**: does this work with nested fields? [Example][io-example] seems
to imply **yes**.

It seems to try to downcast a `options: &dyn Any` into `InspectorOptions`
(TODO check source of `options` parameter of `ui_for_struct`)
If fail, it actually passes a `&() as &dyn Any`, if it succeeds, it will pass
the thing stored in `field` (a `&dyn TypeData` that sometimes can be a `NumberOptions`).
In `ui_for_reflect_with_options`, check if `()`, if so, try to extract the
`ReflectInspectorOptions` from type registry for type it is trying to display.

```rust
// value: &mut dyn Reflect
// options: &dyn Any
if options.is::<()>() {
    if let Some(data) = self
        .type_registry
        .get_type_data::<ReflectInspectorOptions>(Any::type_id(value))
    {
        options = &data.0;
    }
}
```

Later in the function, does the same with `InspectorEguiImpl`, if it fails, it
displays an error (assuming `value` is a `ReflectRef::Value`)

`InspectorEguiImpl` is added to the registry as `TypeData` in [`inspector_egui_impls`]
for a bunch of types.

Note that fields of `InspectorEguiImpl` are function pointers, that themselves
accept a few `&dyn Any` as parameter; One of them is `options`, which is interally
downcasted to relevant type (with a `unwrap_default` in cases where the option
isn't defined).

### `cuicui_richtext` pull design

cuicui_richtext, unlike inspector, doesn't need to navigate the whole structures,
"only" need to extract a specific field. Relieves us of much pain.

> **Note**
> This is untrue in cases where we derive `StylizeOptions` on a struct which
> itself is a field of a resource. This should be left for later TODO(feat)

Still, the display options should be specified by users, I see two approaches:

### A `derive` macro like inspector's `InspectorOptions`

I would name the derive macro `StylizeOptions`.
Preferred over `StyleOptions` because we aren't changing the style (color, size)
of display, but what kind of style the field provides.
Nor `Stylize` because it implies that it is a requirement to be able to refer
to them in format strings.

```rust
#[derive(StylizeOptions, Reflect, Debug, Resource, Default)]
#[reflect(StylizeOptions, Resource)]
struct GameConfig {
  #[style("{:.2}")]
  master_volume: f32,

  #[style("{:.2}")]
  music_volume: f32,

  #[style("{:.2}")]
  effects_volume: f32,

  #[style(modify)]
  difficulty: Difficulty,
}
```

**Question 4**: how could user specify how to display `Difficulty`?

```rust
#[derive(Clone, Copy, Debug, PartialEq, Reflect, Default)]
#[reflect(Default)]
enum Difficulty { Easy, #[default] Normal, Hard }

impl Modify for Difficulty {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        let (color, description) = match self {
            Self::Easy => (Color::BLUE, "Easy"),
            Self::Normal => (Color::ORANGE, "Normal"),
            Self::Hard => (Color::RED, "Hard"),
        };
        text.style.color = color;
        text.value.clear();
        text.value.push_str(description);
        Some(())
    }
    fn clone_dyn(&self) -> ModifyBox { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn Any { self }
    fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
    fn eq_dyn(&self, other: &dyn Modify) -> bool {
        let any = other.as_any();
        let Some(right) = any.downcast_ref::<Self>() else { return false; };
        self == right
    }
}
```

Alternatively, we could define a format trait with an easier derivation, as to
avoid having to make our types `Modify`.

### Inline format strings

This actually sucks. We still have to specify how to print things in two separate
places, it makes things worse since there is all this verbosity around derives.

What about specifying the formatting where it's used?

(check `format_args!` doc)

```
effects volume: {content:$Resources.GameConfig.effects_volume, format:'{:.2}'}
```

> **Warning**
> This requires a way to combine modifiers (Dynamic from content + format)

How would this look like with `Difficulty`?

```
Difficulty: {content:$Resources.GameConfig.difficulty}
```

Say I use `Reflect::debug` or `Reflect::serializable`, this removes from user
ability to control how it shows (which is the whole point!)

I can't specify rust code inside the format string.

If we want to keep format and format string together, we could refer to names
in the format string and provide the name in the call. For example

```rust
let rich_text = RichText::parse(
  "Difficulty: {Content:$Resources.GameConfig.difficulty, Format: 'show_difficulty'}"
)
  .with("show_difficulty", |d| match d {
    Difficulty::Hard => "hard",
    Difficulty::Easy => "easy",
    Difficulty::Normal => "normal",
  })
  .build()
  .unwrap();
```

> **Warning**
> TODO(feat): We should allow somehow to convert `dyn Reflect` to `dyn Modify`
> on top of this beautiful API.

> **Warning**
> TODO(feat): Consider a fallback when the `format` is not specified (like 
> `Reflect::debug` or `Enum::name` etc.)

[`inspector_egui_impls`]: https://github.com/jakobhellermann/bevy-inspector-egui/blob/main/crates/bevy-inspector-egui/src/inspector_egui_impls/mod.rs
[io-example]: https://github.com/jakobhellermann/bevy-inspector-egui/blob/main/crates/bevy-inspector-egui/examples/basic/inspector_options.rs
[`bevy-inspector-egui`]: https://docs.rs/bevy-inspector-egui/0.16.0/bevy_inspector_egui/inspector_options/std_options/struct.NumberOptions.html