# cuici fab derive marcro

This crate implements the `impl_modify` macro. 

`impl_modify` is an attribute macro to define `Modify<I>` with correct
change tracking without too much headache.

# Syntax

See the next section for a detailed explanation with semantic information.

`#[impl_modify]` only works on `impl` block, it should be formatted as a
trait implementation for `Modify<I>` as follow:

Consider the following structs:

```text
struct Color([f32;4]);
impl Color {
  fn as_hsla(&self) -> [f32;4] { self.0 }
  fn hsla(inner: [f32;4]) -> Self { Color(inner) }
}

struct TextStyle {
  color: Color,
}

struct TextSection {
  value: String,
  style: TextStyle,
}
```

```text
use cuicui_fab_derive::impl_modify;

#[impl_modify]
impl Modify<TextSection> for CustomModify {
    // ...
```

The first item (declaration) within that `impl` block should be a type
definition for `Context`. The type definition accepts either 1 or 0 lifetime
parameters:

```text
    // ...
    type Context<'a> = ();
    // ...
```

All other items are function declarations. They can be documented. You can
decorate them with the `modify` attributes.

```text
    // ...
    #[modify(read_write(.style.color))]
    fn shift_hue(hue_offset: f32, color: &mut Color) {
        let mut hsl = color.as_hsla_f32();
        hsl[0] = (hsl[0] + hue_offset) % 360.0;
        *color = Color::hsla(hsl);
    }
    // ...
}
```

## Attributes

All the `modify` attributes are:

- `#[modify(context(value))]`: Pass the context to the parameter named `value`.
   It must be of the same type as `type Context`.
- `#[modify(read(it.path.to.value))]`: Which field of the item (`it` stands
   for item) to read. The last field name (here `value`) is the argument
   used to pass thie value to the function.
- `#[modify(write(it.path.to.value))]`: which field of the item  to update
   with the return value of this function, a function can only have a
   single `write` attribute, and this excludes the use of `read_write`.
- `#[modify(read_write(it.path.to.value))]`:
- `#[modify(write_mut(it.path.to.value))]`: This works like `read_write`
   (the argument to modify is passed as a `&mut`) but we assume that you do
   not read or use the content of the value in the return value. This is
   typically useful when you want to overwrite a string value without
   allocating anything.

```text
#[modify(read(.style.font_size))]
fn some_change(font_size: f32) {
    // ...
}
```

You might want to rename the field name before passing it as argument.
To do so, use the following syntax:

```text
#[modify(read(size = .style.font_size))]
fn some_change(size: f32) {
    // ...
}
```

# How does it look?

You define a `Modify` operating on an arbitrary item, here, we will use the
bevy `TextSection`, element of a `Text` component.

`Modify` itself is a trait, you need a type on which to implement it. Here,
we chose to name our `Modify` type `CustomModify` (ik very creative).

**Do not define `CustomModify` yourself**, `impl_modify` will implement it
for you. See the next section to learn how `CustomModify` looks like.

`CustomModify` operations are defined as functions in the `impl` block.
Those functions have two kinds of arguments:

1. Internal parameters: will be constructors of `CustomModify`.
2. Passed parameters: they are fields of the modiify item `I`,
   here `TextSection`

```text
#[impl_modify]
impl Modify<TextSection> for CustomModify {
    type Context<'a> = GetFont<'a>;
    
}
```

# How does it work?

`impl_modify` creates two enums:

- `CustomModify`: One variant per free function defined in the `impl_modify`
  block.
- `CustomModifyField`: each accessed field, implements `EnumSetType`,
  to be used as `Modify::Field` of `CustomModify`.

`CustomModify` implements `Modify<TextSection>` and has one constructor per
defined function in `impl_modify`. In our last example, we defined:

- `shift_hue(offset: f32)`
- `color(set_to: Color)`
- `font(name: String)`

Therefore, our `CustomModify` will look as follow:

```text
enum CustomModify {
  ShiftHue { offset: f32 },
  Color { set_to: Color },
  Font { name: String },
}
impl CustomModify {
  const fn shift_hue(offset: f32) -> Self {
    Self::ShiftHue { offset }
  }
  const fn color(set_to: Color) -> Self {
    Self::Color { set_to }
  }
  pub const fn font(name: String) -> Self {
    Self::Font { name }
  }
}
```