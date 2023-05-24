//! Provided implementations for the [`Modify<TextPrefab>`] trait for cuicui.
use std::{any::Any, borrow::Cow, fmt};

use anyhow::Error as AnyError;
use bevy::prelude::{trace, FromReflect, Reflect};
use bevy::reflect::ReflectFromReflect;
use bevy::text::TextSection;
use enumset::EnumSet;
use fab::{impl_modify, prefab::Modify};
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

impl Parse for Font {
    const NAME: &'static str = "Font";

    fn parse(input: &str) -> Result<ModifyBox, AnyError> {
        Ok(Box::new(Font(input.to_string())))
    }
}
impl Parse for RelSize {
    const NAME: &'static str = "RelSize";
    fn parse(input: &str) -> Result<ModifyBox, AnyError> {
        Ok(Box::new(RelSize(input.parse()?)))
    }
}
impl Parse for Color {
    const NAME: &'static str = "Color";
    fn parse(input: &str) -> Result<ModifyBox, AnyError> {
        Ok(Box::new(Color(crate::parse::color(input)?)))
    }
}

#[impl_modify]
#[modify(derive(Reflect, PartialEq, Debug, Clone, FromReflect))]
impl Modify<TextSection> for TextModifiers {
    type Context<'a> = GetFont<'a>;

    #[modify(context(ctx), write(.style.font))]
    fn font(path: Cow<'static, str>, ctx: &GetFont) -> anyhow::Result<Handle<Font>> {
        let err = || Errors::FontNotLoaded(self.0.clone());
        trace!("Apply =font=: {path:?}");
        get_font(&self.0).ok_or_else(err)
    }
    #[modify(read_write(.style.font_size))]
    fn rel_size(relative_size: f32, font_size: &mut f32) -> anyhow::Result<()> {
        trace!("Apply =rel_size=: {relative_size:?}");
        *font_size *= relative_size;
        Ok(())
    }
    #[modify(write(.style.color))]
    fn color(new: Color) -> anyhow::Result<Color> {
        trace!("Apply ~COLOR~: {new:?}");
        Ok(new)
    }
    #[modify(write_mut(.value))]
    fn content(new: Cow<'static, str>, value: &mut String) -> anyhow::Result<()> {
        trace!("Apply $CONTENT$: {new:?}");
        value.clear();
        value.push_str(&self.0);
        Ok(())
    }
    #[modify(dynamic_read_write(depends, changes), context(ctx))]
    fn dynamic(boxed: ModifyBox, ctx: &GetFont, it: &mut TextSection) -> anyhow::Result<()> {
        boxed.apply(ctx, it)
    }
}
impl<T: TextModify> From<T> for TextModifiers {
    fn from(value: T) -> Self {
        TextModifiers::Dynamic { depends: value.depends(), changes: value.changes() }
    }
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
