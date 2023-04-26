//! A rich text component managing bevy a single [`Text`].
//!
//! As with the rest of `bevy_cuicui`, you can either build one by parsing
mod color;
mod dynamic;
mod parse;

use std::any::TypeId;
use std::fmt;

use bevy::prelude::Color as BevyColor;
use bevy::{asset::HandleId, prelude::*, utils::HashMap};

use dynamic::{TextContext, TextMod};

pub use parse::Error as ParseError;

pub type DynModifier = Box<dyn TextMod + Send + Sync + 'static>;
pub type Modifiers = HashMap<TypeId, DynModifier>;

/// A font name.
pub struct Font(pub String);
impl TextMod for Font {
    fn apply(&self, ctx: &TextContext, text: &mut TextSection) -> Option<()> {
        text.style.font = ctx.fonts.get_handle(HandleId::from(&self.0));
        Some(())
    }
}

/// Size relative to global text size.
pub struct RelSize(pub f32);
impl TextMod for RelSize {
    fn apply(&self, ctx: &TextContext, text: &mut TextSection) -> Option<()> {
        text.style.font_size = ctx.parent_style.font_size * self.0;
        Some(())
    }
}

/// Color.
pub struct Color(pub BevyColor);
impl TextMod for Color {
    fn apply(&self, _ctx: &TextContext, text: &mut TextSection) -> Option<()> {
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
    fn apply(&self, _ctx: &TextContext, text: &mut TextSection) -> Option<()> {
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

// TODO(text): should have change tracking (might require internal mutability)
// to be precise and extremely limited about what we update.
pub struct Section {
    modifiers: Modifiers,
}
pub struct RichText {
    pub sections: Vec<Section>,
}

impl RichText {
    /// Default cuicui rich text parser. Using a syntax inspired by rust's `format!` macro.
    ///
    /// See [rust doc](https://doc.rust-lang.org/stable/std/fmt/index.html).
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let sections = parse::rich_text(input)?;
        Ok(RichText { sections })
    }
    // TODO(text): consider RichText independent from entity, might control several
    pub fn update(&self, to_update: &mut Text, ctx: &TextContext) {
        let r_len = self.sections.len();
        let p_len = to_update.sections.len();
        if r_len != p_len {
            to_update.sections.resize_with(r_len, default);
        }
        let rich = self.sections.iter();
        let poor = to_update.sections.iter_mut();

        for (to_set, value) in poor.zip(rich) {
            for modifier in value.modifiers.values() {
                modifier.apply(ctx, to_set);
            }
        }
        todo!()
    }
}
