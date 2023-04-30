//! Default implementations of the [`TextMod`] trait for cuicui.
use std::{borrow::Cow, fmt};

use bevy::prelude::Color as BevyColor;
use bevy::prelude::*;
use bevy::reflect::ReflectFromReflect;

use super::{Context, Modify};

macro_rules! common_modify_methods {
    () => {
        fn clone_dyn(&self) -> super::ModifyBox {
            Box::new(self.clone())
        }
    };
}

/// A font name.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Font(pub String);
impl Modify for Font {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new font: {:?}", self.0);
        text.style.font = (ctx.fonts)(&self.0)?;
        Some(())
    }
    common_modify_methods! {}
}

/// Size relative to global text size.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct RelSize(pub f32);
impl Modify for RelSize {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new font size: {:?}", self.0);
        text.style.font_size = ctx.parent_style.font_size * self.0;
        Some(())
    }
    common_modify_methods! {}
}

/// Color.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Color(pub BevyColor);
impl Modify for Color {
    fn apply(&self, _ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new color: {:?}", self.0);
        text.style.color = self.0;
        Some(())
    }
    common_modify_methods! {}
}

/// A section text, may either be preset or extracted.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Content(pub Cow<'static, str>);
impl Modify for Content {
    fn apply(&self, _ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new content: {:?}", self.0);
        text.value.clear();
        text.value.push_str(&self.0);
        Some(())
    }
    common_modify_methods! {}
}
impl<T: fmt::Display> From<T> for Content {
    fn from(value: T) -> Self {
        Content(value.to_string().into())
    }
}

// TODO(text): most likely could use interning for Dynamic section text name.
// this would involve replacing Strings with an enum String|Interned, or private
// background components, otherwise API seems impossible.
/// An [`Modify`] that takes it value from [`Context::bindings`].
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Dynamic {
    pub name: String,
}
impl Dynamic {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
impl Modify for Dynamic {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Get value from binding: {:?}", self.name);
        ctx.bindings?.get(self.name.as_str())?.apply(ctx, text)
    }
    common_modify_methods! {}
}
impl Modify for () {
    fn apply(&self, _: &Context, _: &mut TextSection) -> Option<()> {
        Some(())
    }
    common_modify_methods! {}
}
