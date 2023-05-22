//! Provided implementations for the [`Modify<TextPrefab>`] trait for cuicui.
use std::{any::Any, borrow::Cow, fmt};

use anyhow::Error as AnyError;
use bevy::prelude::{trace, FromReflect, Reflect};
use bevy::reflect::ReflectFromReflect;
use bevy::text::TextSection;
use enumset::EnumSet;
use fab::prefab::Modify;
use thiserror::Error;

use crate::richtext::{self, Field, GetFont};

/// A Boxed [`Modify<TextPrefab>`] trait object, with all necessary bounds to make it work
/// with bevy's [`Resource`] and [`Component`] types.
///
/// [`Resource`]: bevy::prelude::Resource
/// [`Component`]: bevy::prelude::Component
pub type ModifyBox = Box<dyn TextModify + Send + Sync + 'static>;

pub(crate) trait Parse {
    /// The name to use when parsing metadata in the format string.
    ///
    /// **This must be formatted as an identifier** (ie: `[:alpha:_][:alphanum:_]*`).
    /// Otherwise, this won't be parsed correctly.
    ///
    /// By default, this is the name of your type.
    ///
    /// The default implementation should cause a compile time error if `Self`
    /// has generic parameters. In which case, you should provide your own
    /// implementation.
    ///
    /// You may overwrite this method regardless, as long as the return value
    /// is an identifier.
    const NAME: &'static str;

    /// Parse from the string representation of the `metadata` value section
    /// of the format string.
    ///
    /// When parsing a format string, we call `parse` of registered
    /// `Modify` types which name we encounter in the `key` metadata position.
    fn parse(input: &str) -> anyhow::Result<ModifyBox>;
}

macro_rules! common_modify_methods {
    () => {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn eq_dyn(&self, other: &dyn TextModify) -> bool {
            let any = other.as_any();
            let Some(right) = any.downcast_ref::<Self>() else { return false; };
            self == right
        }
        fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use std::fmt::Debug;
            self.fmt(f)
        }
    };
}

#[derive(Debug, Error)]
enum Errors {
    #[error(
        "The font specified in the format string wasn't loaded, \
        hence cannot be used (this is why you ain't seeing anything on screen). \
        The font in question: \"{0}\""
    )]
    FontNotLoaded(String),
}

/// A file path to a font, loaded through other means.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Font(pub String);
impl TextModify for Font {
    fn apply(&self, get_font: &GetFont, text: &mut TextSection) -> Result<(), AnyError> {
        let err = || Errors::FontNotLoaded(self.0.clone());
        trace!("Apply =Font=: {:?}", self.0);
        text.style.font = get_font(&self.0).ok_or_else(err)?;
        Ok(())
    }
    fn depends(&self) -> EnumSet<Field> {
        EnumSet::EMPTY
    }
    fn changes(&self) -> EnumSet<Field> {
        richtext::Field::Font.into()
    }
    common_modify_methods! {}
}
impl Parse for Font {
    const NAME: &'static str = "Font";

    fn parse(input: &str) -> Result<ModifyBox, AnyError> {
        Ok(Box::new(Font(input.to_string())))
    }
}

/// Size relative to parent text size.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct RelSize(pub f32);
impl TextModify for RelSize {
    fn apply(&self, _: &GetFont, text: &mut TextSection) -> Result<(), AnyError> {
        trace!("Apply #RelSize#: {:?}", self.0);
        text.style.font_size *= self.0;
        Ok(())
    }
    fn depends(&self) -> EnumSet<Field> {
        richtext::Field::FontSize.into()
    }
    fn changes(&self) -> EnumSet<Field> {
        richtext::Field::FontSize.into()
    }
    common_modify_methods! {}
}
impl Parse for RelSize {
    const NAME: &'static str = "RelSize";
    fn parse(input: &str) -> Result<ModifyBox, AnyError> {
        Ok(Box::new(RelSize(input.parse()?)))
    }
}

/// Color.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Color(pub bevy::prelude::Color);
impl TextModify for Color {
    fn apply(&self, _: &GetFont, text: &mut TextSection) -> Result<(), AnyError> {
        // println!("Apply new color: {:?}", self.0);
        trace!("Apply ~COLOR~: {:?}", self.0);
        text.style.color = self.0;
        Ok(())
    }
    fn depends(&self) -> EnumSet<Field> {
        EnumSet::EMPTY
    }
    fn changes(&self) -> EnumSet<Field> {
        richtext::Field::Color.into()
    }
    common_modify_methods! {}
}
impl Parse for Color {
    const NAME: &'static str = "Color";
    fn parse(input: &str) -> Result<ModifyBox, AnyError> {
        Ok(Box::new(Color(crate::parse::color(input)?)))
    }
}

/// A section text, may either be preset or extracted.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Content(pub Cow<'static, str>);
impl TextModify for Content {
    fn apply(&self, _: &GetFont, text: &mut TextSection) -> Result<(), AnyError> {
        trace!("Apply $CONTENT$: {:?}", self.0);
        text.value.clear();
        text.value.push_str(&self.0);
        Ok(())
    }
    fn depends(&self) -> EnumSet<Field> {
        EnumSet::EMPTY
    }
    fn changes(&self) -> EnumSet<Field> {
        // TODO(clean): It is not true that it doesnt' changee anything, but
        // Content is special-cased RichText so as to avoid extra storage
        EnumSet::EMPTY
    }
    common_modify_methods! {}
}
impl Parse for Content {
    const NAME: &'static str = "Content";
    fn parse(input: &str) -> Result<ModifyBox, AnyError> {
        Ok(Box::new(Content(input.to_string().into())))
    }
}
impl<T: fmt::Display> From<T> for Content {
    fn from(value: T) -> Self {
        Content(value.to_string().into())
    }
}
pub trait TextModify {
    fn apply(&self, ctx: &GetFont, prefab: &mut TextSection) -> anyhow::Result<()>;
    fn depends(&self) -> EnumSet<Field>;
    fn changes(&self) -> EnumSet<Field>;

    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn TextModify) -> bool;
    fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}
impl Modify<TextSection> for ModifyBox {
    type Field = Field;
    type Context<'a> = GetFont<'a>;

    fn apply(&self, ctx: &GetFont, prefab: &mut TextSection) -> anyhow::Result<()> {
        self.as_ref().apply(ctx, prefab)
    }
    fn depends(&self) -> EnumSet<Field> {
        self.as_ref().depends()
    }
    fn changes(&self) -> EnumSet<Field> {
        self.as_ref().changes()
    }
}

impl PartialEq for dyn TextModify {
    fn eq(&self, other: &Self) -> bool {
        self.eq_dyn(other)
    }
}
impl PartialEq for ModifyBox {
    fn eq(&self, other: &Self) -> bool {
        self.eq_dyn(&**other)
    }
}
impl fmt::Debug for dyn TextModify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_dyn(f)
    }
}
impl fmt::Debug for ModifyBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_dyn(f)
    }
}
