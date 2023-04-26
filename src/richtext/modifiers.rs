//! Default implementations of the [`TextMod`] trait for cuicui.
use std::fmt;

use bevy::prelude::*;
use bevy::{asset::HandleId, prelude::Color as BevyColor};

use super::{Context, TextMod};

/// A font name.
pub struct Font(pub String);
impl TextMod for Font {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        text.style.font = ctx.fonts.get_handle(HandleId::from(&self.0));
        Some(())
    }
}

/// Size relative to global text size.
pub struct RelSize(pub f32);
impl TextMod for RelSize {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        text.style.font_size = ctx.parent_style.font_size * self.0;
        Some(())
    }
}

/// Color.
pub struct Color(pub BevyColor);
impl TextMod for Color {
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
impl TextMod for Content {
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

pub enum Dyn<T> {
    Set(T),
    Ref { name: String },
}
impl<T: TextMod> TextMod for Dyn<T> {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        match self {
            Dyn::Ref { name } => ctx.bindings.get(name.as_str())?.apply(ctx, text),
            Dyn::Set(value) => value.apply(ctx, text),
        }
    }
}
impl TextMod for () {
    fn apply(&self, _: &Context, _: &mut TextSection) -> Option<()> {
        Some(())
    }
}
