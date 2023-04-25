//! A rich text component managing bevy a single [`Text`].
//!
//! As with the rest of `bevy_cuicui`, you can either build one by parsing
mod color;
mod parse;

use std::any::TypeId;

use bevy::prelude::{Color as BevyColor, Font as BevyFont};
use bevy::{asset::HandleId, prelude::*, utils::HashMap};

pub use parse::Error as ParseError;

/// The context used to update [`TextStyle`]s for given bevy [`Text`] sections.
pub struct StyleContext<'a> {
    pub parent_style: TextStyle,
    pub fonts: &'a Assets<BevyFont>,
}

/// A [`TextSection`] style modifier.
///
/// A [`TextSection`] may have an arbitary number of
pub trait StyleMod {
    fn apply(&self, ctx: &StyleContext, style: &mut TextStyle);
}

/// A font name.
pub struct Font(pub String);
impl StyleMod for Font {
    fn apply(&self, ctx: &StyleContext, style: &mut TextStyle) {
        style.font = ctx.fonts.get_handle(HandleId::from(&self.0));
    }
}

/// Size relative to global text size.
pub struct RelSize(pub f32);
impl StyleMod for RelSize {
    fn apply(&self, ctx: &StyleContext, style: &mut TextStyle) {
        style.font_size = ctx.parent_style.font_size * self.0;
    }
}

/// Color.
pub struct Color(pub BevyColor);
impl StyleMod for Color {
    fn apply(&self, _ctx: &StyleContext, style: &mut TextStyle) {
        style.color = self.0;
    }
}

// TODO(text): most likely could use interning for Dynamic section text name.
// this would involve replacing Strings with an enum String|Interned, or private
// background components, otherwise API seems impossible.
/// A section text, may either be preset or extracted.
pub enum Content {
    Dynamic { name: String },
    Static(String),
}
// TODO(text): should have change tracking (might require internal mutability)
// to be precise and extremely limited about what we update.
#[derive(Component)]
pub struct Section {
    modifiers: HashMap<TypeId, Box<dyn StyleMod + Send + Sync + 'static>>,
    content: Content,
}
pub struct RichText {
    pub root_style: TextStyle,
    pub sections: Vec<Section>,
}

impl RichText {
    /// Default cuicui rich text parser. Using a syntax inspired by
    pub fn parse(def: &str) -> Result<Self, ParseError> {
        todo!()
    }
    // TODO(text): consider RichText independent from entity, might control several
    pub fn update(&self, to_update: &mut Text) {}
}
