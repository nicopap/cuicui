//! Provided implementations for the [`Modify`] trait for cuicui.
use std::{any::Any, any::TypeId, borrow::Cow, fmt};

use anyhow::Error as AnyError;
use bevy::prelude::{FromReflect, Reflect, TextSection};
use bevy::reflect::ReflectFromReflect;
use thiserror::Error;

use crate::{modify::Context, IntoModify, Modify, ModifyBox};

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
    #[error(
        "A format string requires a dynamic binding named \"{0}\", \
        but it isn't bound in the given context."
    )]
    BindingNotInContext(String),
    #[error(
        "A format string requires a dynamic type binding of \"{0}\", \
        but it isn't bound in the given context."
    )]
    BindingTypeNotInContext(String),
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
    fn parse(input: &str) -> Result<ModifyBox, AnyError>
    where
        Self: Sized,
    {
        Ok(Box::new(Font(input.to_string())))
    }
    common_modify_methods! {}
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
    fn parse(input: &str) -> Result<ModifyBox, AnyError>
    where
        Self: Sized,
    {
        Ok(Box::new(RelSize(input.parse()?)))
    }
    common_modify_methods! {}
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
    fn parse(input: &str) -> Result<ModifyBox, AnyError>
    where
        Self: Sized,
    {
        Ok(Box::new(Color(crate::parse::color(input)?)))
    }
    common_modify_methods! {}
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
        // println!("Apply new content: {:?}", self.0);
        text.value.clear();
        text.value.push_str(&self.0);
        Ok(())
    }
    fn parse(input: &str) -> Result<ModifyBox, AnyError>
    where
        Self: Sized,
    {
        Ok(Box::new(Content(input.to_string().into())))
    }
    common_modify_methods! {}
}
impl<T: fmt::Display> From<T> for Content {
    fn from(value: T) -> Self {
        Content(value.to_string().into())
    }
}

// TODO(perf): most likely could use interning for Dynamic section text name.
// this would involve replacing Strings with an enum String|Interned, or private
// background components, otherwise API seems impossible.
/// An [`Modify`] that takes it value from [`Context::bindings`].
#[derive(PartialEq, Debug, Clone)]
pub enum Dynamic {
    ByName(String),
    // TODO(clean): remove `TypeId` here, since it is necessarily associated
    // with a `TypeId` when inserted into the `Modifiers` map.
    // Probably need to add the `TypeId` to `Context`.
    ByType(TypeId),
}
impl Dynamic {
    pub fn new(name: String) -> Self {
        Dynamic::ByName(name)
    }
}
impl Modify for Dynamic {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Result<(), AnyError> {
        // println!("Get value from binding: {:?}", self.name);
        let modifier = match self {
            Dynamic::ByType(type_id) => {
                let ty_name = || {
                    let info = ctx.registry?.get_type_info(*type_id)?;
                    Some(info.type_name().to_string())
                };
                let ty_badname = || format!("Unregistered type: {type_id:?}");
                let err = || Errors::BindingTypeNotInContext(ty_name().unwrap_or_else(ty_badname));
                let run = || ctx.type_bindings?.get(type_id);
                run().ok_or_else(err)
            }
            Dynamic::ByName(name) => {
                let local_binding = || ctx.bindings?.get(&**name);
                let world_binding = || ctx.world_bindings?.get(&**name);
                let err = || Errors::BindingNotInContext(name.clone());
                local_binding().or_else(world_binding).ok_or_else(err)
            }
        };
        modifier?.apply(ctx, text)
    }
    fn name() -> Option<&'static str>
    where
        Self: Sized,
    {
        None
    }
    common_modify_methods! {}
}
impl Modify for () {
    fn apply(&self, _: &Context, _: &mut TextSection) -> Result<(), AnyError> {
        Ok(())
    }
    fn name() -> Option<&'static str>
    where
        Self: Sized,
    {
        None
    }
    common_modify_methods! {}
}
