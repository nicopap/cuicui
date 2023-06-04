//! [`Modify`] for richtext.
use std::{any::Any, borrow::Cow, fmt};

use bevy::asset::HandleId;
use bevy::prelude::{trace, Assets, Color, Handle};
use bevy::text::{Font, Text, TextSection};
use enumset::EnumSet;
use fab::modify::Indexed;
use fab::{impl_modify, modify::Modify};
use fab_parse::{Deps, Parsable};

/// A Boxed [`TextModifier`]. This allows you to extend [`Modifier`] with your
/// own modifiers.
///
/// [`Resource`]: bevy::prelude::Resource
/// [`Component`]: bevy::prelude::Component
pub type ModifyBox = Box<dyn TextModify + Send + Sync + 'static>;

#[derive(Default, Clone, Copy)]
pub struct GetFont<'a>(Option<&'a Assets<Font>>);
impl<'a> GetFont<'a> {
    pub fn new(assets: &'a Assets<Font>) -> Self {
        GetFont(Some(assets))
    }
    pub fn get(&self, name: &str) -> Option<Handle<Font>> {
        self.0.map(|a| a.get_handle(HandleId::from(name)))
    }
}

impl Indexed<Modifier> for Text {
    fn get_mut(&mut self, index: usize) -> Option<&mut TextSection> {
        self.sections.as_mut_slice().get_mut(index)
    }
}

/// Operations on bevy [`TextSection`]s.
///
/// You can create your own operations. At the cost of storing them as a [`ModifyBox`]
/// and having to be careful about what you update. You create such a `Modifier`
/// using [`Modifier::Dynamic`].
#[impl_modify(cuicui_fab_path = fab, no_derive(Debug))]
#[derive(PartialEq)]
impl Modify for Modifier {
    type Context<'a> = GetFont<'a>;
    type Item = TextSection;
    type Items = Text;

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
impl Parsable for Modifier {
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

    fn parse(name: &str, input: &str) -> Result<Self, Self::Err> {
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
    fn apply(&self, ctx: &GetFont, section: &mut TextSection);
    fn depends(&self) -> EnumSet<ModifierField>;
    fn changes(&self) -> EnumSet<ModifierField>;

    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn TextModify) -> bool;
    fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn clone_dyn(&self) -> ModifyBox;
}
impl Clone for ModifyBox {
    fn clone(&self) -> Self {
        self.clone_dyn()
    }
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
