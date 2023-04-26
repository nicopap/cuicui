//! Default implementations of the [`TextMod`] trait for cuicui.
use std::fmt;

use bevy::prelude::*;
use bevy::{asset::HandleId, prelude::Color as BevyColor};

use super::{Context, Modify};

/// A font name.
pub struct Font(pub String);
impl Modify for Font {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        text.style.font = ctx.fonts.get_handle(HandleId::from(&self.0));
        Some(())
    }
}

/// Size relative to global text size.
pub struct RelSize(pub f32);
impl Modify for RelSize {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        text.style.font_size = ctx.parent_style.font_size * self.0;
        Some(())
    }
}

/// Color.
pub struct Color(pub BevyColor);
impl Modify for Color {
    fn apply(&self, _ctx: &Context, text: &mut TextSection) -> Option<()> {
        text.style.color = self.0;
        Some(())
    }
}

// TODO(text): most likely could use interning for Dynamic section text name.
// this would involve replacing Strings with an enum String|Interned, or private
// background components, otherwise API seems impossible.
/// A section text, may either be preset or extracted.
pub struct Content(pub String);
impl Modify for Content {
    fn apply(&self, _ctx: &Context, text: &mut TextSection) -> Option<()> {
        text.value.clear();
        text.value.push_str(&self.0);
        Some(())
    }
}
impl<T: fmt::Display> From<T> for Content {
    fn from(value: T) -> Self {
        Content(value.to_string())
    }
}

/// An [`ApplySection`] that takes it value from [`Context::bindings`].
pub struct Dynamic {
    pub name: String,
}
impl Modify for Dynamic {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        ctx.bindings.get(self.name.as_str())?.apply(ctx, text)
    }
}
impl Modify for () {
    fn apply(&self, _: &Context, _: &mut TextSection) -> Option<()> {
        Some(())
    }
}
