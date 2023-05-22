# Track individual fields

Problem: We want to modify only fields of `Modify` implementors, and declare
those changes statically, not in a separate method.

### Idea 1: a macro

```rust
#[impl_modify]
impl Modify<TextSection> for CustomModify {
  type Context<'a> = GetFont<'a>;
  
  // Variant 1: Works a bit like wgsl, but it is fairly verbose and likely to
  // mess with RA and syntax highlight
  fn shift_hue(
    offset: f32,
    #[path(item.style.color)]
    color: Color,
  ) -> #[path(item.style.color)] Color
  {
    let hsl = color.as_hsla_f32();
    hsl[0] = (hsl[0] + offset) % 360.0;
    Color::hsla(hsl[0], hsl[1], hsl[2], hsl[3])
  }
  // Variant 2: declare in #[…] which fields are read, how to pass them
  // to the function. The argument with the last field name will be given
  // the field value.
  // - If read_write, it is passed as Mut<_>: Potential advantage is we could
  //   let users declare they didn't change the value.
  // - If need to rename field, can use syntax `foo = item.path.to.foozz`.
  // - The `writes` is where the value is written to. By comparing the previous
  //   and new value, we could reduce update rate.
  // - Might be harder to implement
  #[reads(item.style.color)]
  #[writes(item.style.color)]
  fn shift_hue(offset: f32, color: Color) -> Color;
  
  #[read_write(item.style.color)]
  #[read_write(color = item.style.color)]
  fn shift_hue(offset: f32, color: Mut<Color>);

  // Variant 3:
  // The arguments are passed implicitly here, not declared as method arguments.
  // might mess with RA, and more difficult to understand
  #[reads(.style.color)] // #[reads(.style.color as color)]
  #[writes(.style.color)]
  fn shift_hue(offset: f32) -> Color;

  fn color(color: Color) -> #[path(.style.color)] Color {
    color
  }
  
  pub fn font(name: String, ctx: #[ctx] GetFont) -> #[path(.style.font)] Handle<Font> {
      trace!("Apply =Font=: {:?}", self.0);
      ctx.get_font(&self.0).unwrap()
  }
}
```

Should generate

```rust
#[derive(EnumSetType)]
enum CustomModifyField {
  StyleColor,
  StyleFont,
}
enum CustomModify {
  ShiftHue { offset: f32 },
  Color { color: Color },
  Font { name: String },
}
impl Modify<TextSection> for CustomModify {
    type Field = CustomModifyField;
    type Context<'a> = GetFont<'a>;

    fn apply(&self, ctx: &Self::Context<'_>, prefab: &mut TextSection) -> anyhow::Result<()> {
      match self {
        Self::ShiftHue { offset } => prefab.style.color = {
          let color = prefab.style.color;
          let hsl = color.as_hsla_f32();
          hsl[0] = (hsl[0] + offset) % 360.0;
          Color::hsla(hsl[0], hsl[1], hsl[2], hsl[3])
        },
        Self::Color { color } => prefab.style.color = {
          color
        },
        Self::Font { name } => prefab.style.font = {
          let ctx = ctx;
          trace!("Apply =Font=: {:?}", self.0);
          ctx.get_font(&self.0).unwrap()
      }
    }
    fn depends(&self) -> EnumSet<Self::Field> {
      match self {
        Self::ShiftHue { .. } => Self::Field::StyleColor,
        Self::Color { .. } => EnumSet::EMPTY,
        Self::Font { .. } => EnumSet::EMPTY,
      }
    }
    fn changes(&self) -> EnumSet<Self::Field> {
      match self {
        Self::ShiftHue { .. } => Self::Field::StyleColor,
        Self::Color { .. } => Self::Field::StyleColor,
        Self::Font { .. } => Self::Field::StyleFont,
      }
    }
}
impl CustomModify {
  const fn shift_hue(offset: f32) -> Self {
    Self::ShiftHue { offset }
  }
  const fn color(color: Color) -> Self {
    Self::Color { color }
  }
  pub const fn font(name: String) -> Self {
    Self::Font { name }
  }
}
```

### The fors and againsts

The bevy community will _hate_ this use of macros, since it's very magic,
even though the code it generates is relatively trivial.

I can't concieve of how to avoid it though. Central elements of this macro are:

1. Creating the `_FOO_Field` enum, to list all things it can access
2. Declaring which fields are accessed by each modify. It guarentees that only
   the bits that are declared as modified are indeed modified.
3. Automatically building `_FOO_` as an `enum` where each variant is a separate
   operation on the item.
  - No need to define each variants and have 3 different matches