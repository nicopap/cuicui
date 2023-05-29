//! Provided implementations for the [`Modify<TextPrefab>`] trait for cuicui.
use std::fmt::Write;
use std::{any::Any, borrow::Cow, fmt};

use bevy::prelude::{trace, Color, Handle};
use bevy::text::{Font, TextSection};
use enumset::EnumSet;
use fab::{impl_modify, prefab::Modify};

use crate::richtext::GetFont;

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
#[derive(PartialEq, Debug)]
impl Modify<TextSection> for Modifier {
    type Context<'a> = GetFont<'a>;

    /// Set the font to provided `path`.
    #[modify(context(get_font), write(.style.font))]
    fn font(path: &Cow<'static, str>, get_font: &GetFont) -> Handle<Font> {
        trace!("Apply =font=: {path:?}");
        get_font.get(path).unwrap_or_default()
    }
    /// Increase the font size relative to the current section.
    #[modify(read_write(.style.font_size))]
    fn rel_size(relative_size: f32, font_size: &mut f32) {
        trace!("Apply :rel_size: {relative_size:?}");
        *font_size *= relative_size;
    }
    /// Increase the font size relative to the current section.
    #[modify(write(.style.font_size))]
    fn font_size(size: f32) -> f32 {
        size
    }
    /// Set the color of the [`TextSection`] to `statik`.
    #[modify(write(.style.color))]
    fn color(statik: Color) -> Color {
        trace!("Apply ~COLOR~: {statik:?}");
        statik
    }
    /// Offset the color's Hue by `offset`.
    #[modify(read_write(.style.color))]
    fn hue_offset(offset: f32, color: &mut Color) {
        trace!("Apply ~HueOffset~: {offset:?}");
        let mut hsl = color.as_hsla_f32();
        hsl[0] = (hsl[0] + offset) % 360.0;
        *color = Color::hsla(hsl[0], hsl[1], hsl[2], hsl[3]);
    }
    /// Set the text content of the [`TextSection`] to `statik`.
    #[modify(write_mut(.value))]
    fn content(statik: &Cow<'static, str>, value: &mut String) {
        trace!("Apply $CONTENT$: {statik:?}");
        value.clear();
        value.push_str(statik);
    }
    /// Use an arbitrary [`ModifyBox`] to modify this section.
    #[modify(dynamic_read_write(depends, changes, item), context(ctx))]
    fn dynamic(boxed: &ModifyBox, ctx: &GetFont, item: &mut TextSection) {
        boxed.apply(ctx, item);
    }
}
impl Modifier {
    /// Set this [`Modifier`] to [`Modifier::Content`].
    ///
    /// Note that this **doesn't allocate** if `self` is already [`Modifier::Content`].
    pub fn overwrite_content(&mut self, new_content: &impl fmt::Display) {
        if let Modifier::Content { statik } = self {
            let statik = statik.to_mut();
            statik.clear();
            write!(statik, "{new_content}").unwrap();
        } else {
            *self = Modifier::content(new_content.to_string().into());
        }
    }
    pub fn parse(name: &str, input: &str) -> anyhow::Result<Self> {
        match name {
            n if n == "Font" => Ok(Self::font(input.to_string().into())),
            n if n == "RelSize" => Ok(Self::rel_size(input.parse()?)),
            n if n == "Color" => Ok(Self::color(crate::parse::color(input)?)),
            n if n == "Content" => Ok(Self::content(input.to_string().into())),
            // TODO(err): nice struct instead of anyhow
            n => Err(anyhow::anyhow!(format!("{n} is not a parseable modifier"))),
        }
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
