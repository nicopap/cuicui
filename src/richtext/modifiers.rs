//! Default implementations of the [`TextMod`] trait for cuicui.
use std::fmt;

use bevy::prelude::Color as BevyColor;
use bevy::prelude::*;

use super::{Context, Modify, ModifyBox};

macro_rules! debug_methods {
    () => {
        fn as_any(&self) -> Option<&dyn std::any::Any> {
            Some(self)
        }
        fn cmp(&self, other: &dyn Modify) -> bool {
            let Some(right) = other.as_any() else { return false};
            let Some(right) = right.downcast_ref::<Self>() else { return false};
            self == right
        }
        fn debug_show(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use std::fmt::Debug;
            self.fmt(f)
        }
        fn clone_dyn(&self) -> ModifyBox {
            Box::new(self.clone())
        }
    };
}
/// A font name.
#[derive(PartialEq, Debug, Clone)]
pub struct Font(pub String);
impl Modify for Font {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new font: {:?}", self.0);
        text.style.font = (ctx.fonts)(&self.0)?;
        Some(())
    }
    debug_methods! {}
}

/// Size relative to global text size.
#[derive(PartialEq, Debug, Clone)]
pub struct RelSize(pub f32);
impl Modify for RelSize {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new font size: {:?}", self.0);
        text.style.font_size = ctx.parent_style.font_size * self.0;
        Some(())
    }
    debug_methods! {}
}

/// Color.
#[derive(PartialEq, Debug, Clone)]
pub struct Color(pub BevyColor);
impl Modify for Color {
    fn apply(&self, _ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new color: {:?}", self.0);
        text.style.color = self.0;
        Some(())
    }
    debug_methods! {}
}

/// A section text, may either be preset or extracted.
#[derive(PartialEq, Debug, Clone)]
pub struct Content(pub String);
impl Modify for Content {
    fn apply(&self, _ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new content: {:?}", self.0);
        text.value.clear();
        text.value.push_str(&self.0);
        Some(())
    }
    debug_methods! {}
}
impl<T: fmt::Display> From<T> for Content {
    fn from(value: T) -> Self {
        Content(value.to_string())
    }
}

// TODO(text): most likely could use interning for Dynamic section text name.
// this would involve replacing Strings with an enum String|Interned, or private
// background components, otherwise API seems impossible.
/// An [`Modify`] that takes it value from [`Context::bindings`].
#[derive(PartialEq, Debug, Clone)]
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
    debug_methods! {}
}
impl Modify for () {
    fn apply(&self, _: &Context, _: &mut TextSection) -> Option<()> {
        Some(())
    }
    debug_methods! {}
}
