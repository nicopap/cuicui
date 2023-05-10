//! Provided implementations for the [`Modify`] trait for cuicui.
use std::{any::Any, borrow::Cow, fmt};

use anyhow::Error as AnyError;
use bevy::prelude::{trace, FromReflect, Reflect, TextSection};
use bevy::reflect::ReflectFromReflect;
use thiserror::Error;

use crate::{
    modify, modify::BindingId, modify::Context, modify::DependsOn, IntoModify, Modify, ModifyBox,
};

macro_rules! common_modify_methods {
    () => {
        fn clone_dyn(&self) -> super::ModifyBox {
            Box::new(self.clone())
        }
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
        // println!("Apply new font: {:?}", self.0);
        text.style.font = (ctx.fonts)(&self.0).ok_or_else(err)?;
        Ok(())
    }
    fn depends_on(&self) -> Vec<DependsOn> {
        vec![DependsOn::Fonts]
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
        // println!("Apply new font size: {:?}", self.0);
        text.style.font_size = ctx.parent_style.font_size * self.0;
        Ok(())
    }
    fn depends_on(&self) -> Vec<DependsOn> {
        vec![DependsOn::StyleFontSize]
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
        text.style.color = self.0;
        Ok(())
    }
    fn depends_on(&self) -> Vec<DependsOn> {
        Vec::new()
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
        trace!("Apply new content: {:?}", self.0);
        text.value.clear();
        text.value.push_str(&self.0);
        Ok(())
    }
    fn depends_on(&self) -> Vec<DependsOn> {
        Vec::new()
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

/// An [`Modify`] that takes it value from [`Context::bindings`].
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Dynamic(pub(crate) BindingId);
impl Modify for Dynamic {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Result<(), AnyError> {
        // println!("Get value from binding: {:?}", self.name);
        let Some(modifier) = ctx.get_binding(self.0) else { return Ok(()) };
        modifier.apply(ctx, text)
    }
    fn depends_on(&self) -> Vec<modify::DependsOn> {
        // TODO(bug): problem: this also depends on the dependencies of the `Modify` this resolves to.
        vec![DependsOn::Binding(self.0)]
    }
    common_modify_methods! {}
}
impl Modify for () {
    fn apply(&self, _: &Context, _: &mut TextSection) -> Result<(), AnyError> {
        Ok(())
    }
    fn depends_on(&self) -> Vec<modify::DependsOn> {
        Vec::new()
    }
    common_modify_methods! {}
}
