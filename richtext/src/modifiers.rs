//! Provided implementations for the [`Modify<TextPrefab>`] trait for cuicui.
use std::{any::Any, borrow::Cow, fmt};

use bevy::prelude::{trace, Color, Handle};
use bevy::text::{Font, TextSection};
use enumset::EnumSet;
use fab::{impl_modify, prefab::Modify};
use fab_parse::{Deps, ParsablePrefab};

use crate::richtext::{GetFont, TextPrefab};

/// A Boxed [`Modify<TextPrefab>`] trait object, with all necessary bounds to make it work
/// with bevy's [`Resource`] and [`Component`] types.
///
/// [`Resource`]: bevy::prelude::Resource
/// [`Component`]: bevy::prelude::Component
pub type ModifyBox = Box<dyn TextModify + Send + Sync + 'static>;

/// Operations on bevy [`TextSection`]s.
///
/// You typically get a [`RichText`] from parsing a [format string]. The modifiers
/// are then managed by [`RichText`].
///
/// You can create your own operations. At the cost of storing them as a [`ModifyBox`]
/// and having to be careful about what you update. You create such a `Modifier`
/// using [`Modifier::Dynamic`].
///
/// [`RichText`]: crate::RichText
/// [format string]: https://github.com/nicopap/cuicui/blob/main/design_doc/richtext/informal_grammar.md
#[impl_modify(cuicui_fab_path = fab)]
#[derive(PartialEq)]
impl Modify<TextSection> for Modifier {
    type Context<'a> = GetFont<'a>;

    /// Set the font to provided `path`.
    #[modify(context(get_font), write(.style.font))]
    pub fn font(path: &Cow<'static, str>, get_font: &GetFont) -> Handle<Font> {
        trace!("Apply =font=: {path:?}");
        get_font.get(path).unwrap_or_default()
    }
    /// Increase the font size relative to the current section.
    #[modify(read_write(.style.font_size))]
    pub fn rel_size(relative_size: f32, font_size: &mut f32) {
        trace!("Apply :rel_size: {relative_size:?}");
        *font_size *= relative_size;
    }
    /// Set font size to `size`.
    #[modify(write(.style.font_size))]
    pub fn font_size(size: f32) -> f32 {
        size
    }
    /// Set the color of the [`TextSection`] to `statik`.
    #[modify(write(.style.color))]
    pub fn color(statik: Color) -> Color {
        trace!("Apply ~COLOR~: {statik:?}");
        statik
    }
    /// Offset the color's Hue by `offset`.
    #[modify(read_write(.style.color))]
    pub fn hue_offset(offset: f32, color: &mut Color) {
        trace!("Apply ~HueOffset~: {offset:?}");
        let mut hsl = color.as_hsla_f32();
        hsl[0] = (hsl[0] + offset) % 360.0;
        *color = Color::hsla(hsl[0], hsl[1], hsl[2], hsl[3]);
    }
    /// Set the text content of the [`TextSection`] to `statik`.
    #[modify(write_mut(.value))]
    pub fn content(statik: &Cow<'static, str>, value: &mut String) {
        trace!("Apply $CONTENT$: {statik:?}");
        value.clear();
        value.push_str(statik);
    }
    /// Use an arbitrary [`ModifyBox`] to modify this section.
    #[modify(dynamic_read_write(depends, changes, item), context(ctx))]
    pub fn dynamic(boxed: &ModifyBox, ctx: &GetFont, item: &mut TextSection) {
        boxed.apply(ctx, item);
    }
}
impl fmt::Debug for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Modifier::Font { path } => f.debug_tuple("Font").field(path).finish(),
            Modifier::RelSize { relative_size } => {
                f.debug_tuple("Size^").field(relative_size).finish()
            }
            Modifier::FontSize { size } => f.debug_tuple("FontSize").field(size).finish(),
            Modifier::Color { statik } => f.debug_tuple("Color").field(statik).finish(),
            Modifier::HueOffset { offset } => f.debug_tuple("Hue>").field(offset).finish(),
            Modifier::Content { statik } => f.debug_tuple("Text").field(statik).finish(),
            Modifier::Dynamic { boxed, .. } => write!(f, "{boxed:?}"),
        }
    }
}
impl ParsablePrefab for TextPrefab {
    type Err = anyhow::Error;

    /// Returns the (depends, changes) field set of modifier named `name`.
    fn dependencies_of(name: &str) -> Deps<ModifierField> {
        let mut depends = EnumSet::EMPTY;

        let changes = match name {
            "Font" => Modifier::font_changes(),
            "Color" => Modifier::color_changes(),
            "Content" => Modifier::content_changes(),
            "FontSize" => Modifier::font_size_changes(),
            "RelSize" => {
                depends = Modifier::rel_size_depends();
                Modifier::rel_size_changes()
            }
            "HueOffset" => {
                depends = Modifier::hue_offset_depends();
                Modifier::hue_offset_changes()
            }
            _ => return Deps::NoneWithName,
        };
        Deps::Some { changes, depends }
    }

    fn parse(name: &str, input: &str) -> Result<Self::Modify, Self::Err> {
        match name {
            "Font" => Ok(Modifier::font(input.to_string().into())),
            "FontSize" => Ok(Modifier::font_size(input.parse()?)),
            "RelSize" => Ok(Modifier::rel_size(input.parse()?)),
            "Color" => Ok(Modifier::color(crate::color::parse(input)?)),
            "HueOffset" => Ok(Modifier::hue_offset(input.parse()?)),
            "Content" => Ok(Modifier::content(input.to_string().into())),
            // TODO(err): nice struct instead of anyhow
            n => Err(anyhow::anyhow!(format!("{n} is not a parseable modifier"))),
        }
    }
}

impl fmt::Write for Modifier {
    /// Set this [`Modifier`] to [`Modifier::Content`].
    ///
    /// Note that this **doesn't allocate** if `self` is already [`Modifier::Content`].
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Modifier::Content { statik } = self {
            let statik = statik.to_mut();
            statik.clear();
            statik.push_str(s);
        } else {
            *self = Modifier::content(s.to_string().into());
        }
        Ok(())
    }
}
impl From<String> for Modifier {
    fn from(value: String) -> Self {
        Modifier::content(value.into())
    }
}
impl<T: TextModify + Send + Sync + 'static> From<T> for Modifier {
    fn from(value: T) -> Self {
        Modifier::Dynamic {
            depends: value.depends(),
            changes: value.changes(),
            boxed: Box::new(value),
        }
    }
}

pub trait TextModify {
    fn apply(&self, ctx: &GetFont, prefab: &mut TextSection);
    fn depends(&self) -> EnumSet<ModifierField>;
    fn changes(&self) -> EnumSet<ModifierField>;

    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn TextModify) -> bool;
    fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}
impl PartialEq for ModifyBox {
    fn eq(&self, other: &Self) -> bool {
        self.eq_dyn(&**other)
    }
}
impl PartialEq<&Self> for ModifyBox {
    fn eq(&self, other: &&Self) -> bool {
        self.eq_dyn(&***other)
    }
}
impl fmt::Debug for ModifyBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_dyn(f)
    }
}
