//! Provided implementations for the [`Modify`] trait for cuicui.
use std::{any::Any, borrow::Cow, fmt};

use anyhow::Error as AnyError;
use bevy::prelude::{trace, FromReflect, Reflect, TextSection};
use bevy::reflect::ReflectFromReflect;
use enumset::EnumSet;
use thiserror::Error;

use crate::modify::{Change, Context};
use crate::{modify, IntoModify, Modify, ModifyBox};

macro_rules! common_modify_methods {
    () => {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn eq_dyn(&self, other: &dyn Modify) -> bool {
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
impl Modify for Font {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Result<(), AnyError> {
        let err = || Errors::FontNotLoaded(self.0.clone());
        trace!("Apply =Font=: {:?}", self.0);
        text.style.font = (ctx.fonts)(&self.0).ok_or_else(err)?;
        Ok(())
    }
    fn depends(&self) -> EnumSet<Change> {
        EnumSet::EMPTY
    }
    fn changes(&self) -> EnumSet<Change> {
        Change::Font.into()
    }
    common_modify_methods! {}
}
impl modify::Parse for Font {
    const NAME: &'static str = "Font";

    fn parse(input: &str) -> Result<ModifyBox, AnyError> {
        Ok(Box::new(Font(input.to_string())))
    }
}

/// Size relative to global text size.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct RelSize(pub f32);
impl Modify for RelSize {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Result<(), AnyError> {
        trace!("Apply #RelSize#: {:?}", self.0);
        text.style.font_size = ctx.parent_style.font_size * self.0;
        Ok(())
    }
    fn depends(&self) -> EnumSet<Change> {
        Change::FontSize.into()
    }
    fn changes(&self) -> EnumSet<Change> {
        Change::FontSize.into()
    }
    common_modify_methods! {}
}
impl modify::Parse for RelSize {
    const NAME: &'static str = "RelSize";
    fn parse(input: &str) -> Result<ModifyBox, AnyError> {
        Ok(Box::new(RelSize(input.parse()?)))
    }
}

/// Color.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Color(pub bevy::prelude::Color);
impl Modify for Color {
    fn apply(&self, _ctx: &Context, text: &mut TextSection) -> Result<(), AnyError> {
        // println!("Apply new color: {:?}", self.0);
        trace!("Apply ~COLOR~: {:?}", self.0);
        text.style.color = self.0;
        Ok(())
    }
    fn depends(&self) -> EnumSet<Change> {
        EnumSet::EMPTY
    }
    fn changes(&self) -> EnumSet<Change> {
        Change::Color.into()
    }
    common_modify_methods! {}
}
impl modify::Parse for Color {
    const NAME: &'static str = "Color";
    fn parse(input: &str) -> Result<ModifyBox, AnyError> {
        Ok(Box::new(Color(crate::parse::color(input)?)))
    }
}
impl IntoModify for bevy::prelude::Color {
    fn into_modify(self) -> ModifyBox {
        Box::new(Color(self))
    }
}

/// A section text, may either be preset or extracted.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Content(pub Cow<'static, str>);
impl Modify for Content {
    fn apply(&self, _ctx: &Context, text: &mut TextSection) -> Result<(), AnyError> {
        trace!("Apply $CONTENT$: {:?}", self.0);
        text.value.clear();
        text.value.push_str(&self.0);
        Ok(())
    }
    fn depends(&self) -> EnumSet<Change> {
        EnumSet::EMPTY
    }
    fn changes(&self) -> EnumSet<Change> {
        // TODO(clean): It is not true that it doesnt' changee anything, but
        // Content is special-cased RichText so as to avoid extra storage
        EnumSet::EMPTY
    }
    common_modify_methods! {}
}
impl modify::Parse for Content {
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
