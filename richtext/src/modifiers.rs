//! [`Modify`] for richtext.

#[cfg(feature = "cresustext")]
mod cresus_impl;
#[cfg(feature = "richtext")]
mod rich_impl;

use std::{any::Any, fmt};

use bevy::asset::HandleId;
use bevy::prelude::{Assets, Handle};
use bevy::text::Font;
use enumset::EnumSet;
use fab_parse::{Deps, Parsable};

#[cfg(feature = "cresustext")]
pub use cresus_impl::{Modifier, ModifierField, ModifierItem, ModifierQuery, Sections};
#[cfg(feature = "richtext")]
pub use rich_impl::{Modifier, ModifierField};

/// A Boxed [`TextModify`]. This allows you to extend [`Modifier`] with your
/// own modifiers.
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
    #[cfg(feature = "richtext")]
    fn apply(&self, ctx: &GetFont, section: &mut bevy::text::TextSection);
    #[cfg(feature = "cresustext")]
    fn apply(&self, ctx: &GetFont, item: ModifierItem);
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
