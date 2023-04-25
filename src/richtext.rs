//! A rich text component managing bevy a single [`Text`].
//!
//! As with the rest of `bevy_cuicui`, you can either build one by parsing
use std::any::TypeId;
use std::fmt;
use std::marker::PhantomData;

use thiserror::Error;
use bevy::prelude::{Color as BevyColor, Font as BevyFont};
use bevy::{asset::HandleId, prelude::*, utils::HashMap};

pub struct StyleContext<'a> {
    pub parent_style: TextStyle,
    pub fonts: &'a Assets<BevyFont>,
}
pub trait Style {
    fn apply(&self, ctx: &StyleContext, style: &mut TextStyle);
}

/// A font name.
pub struct Font(pub String);
impl Style for Font {
    fn apply(&self, ctx: &StyleContext, style: &mut TextStyle) {
        style.font = ctx.fonts.get_handle(HandleId::from(&self.0));
    }
}

/// Size relative to global text size.
pub struct RelSize(pub f32);
impl Style for RelSize {
    fn apply(&self, ctx: &StyleContext, style: &mut TextStyle) {
        style.font_size = ctx.parent_style.font_size * self.0;
    }
}

/// Color.
pub struct Color(pub BevyColor);
impl Style for Color {
    fn apply(&self, _ctx: &StyleContext, style: &mut TextStyle) {
        style.color = self.0;
    }
}

// TODO(text): most likely could use interning for Dynamic section text name.
// this would involve replacing Strings with an enum String|Interned, or private
// background components, otherwise API seems impossible.
/// A section text, may either be preset or extracted.
pub enum SectionText {
    Dynamic {
        name: String,
        show: fn(&dyn fmt::Display) -> String,
    },
    Static(String),
}
// TODO(text): should have change tracking (might require internal mutability)
// to be precise and extremely limited about what we update.
#[derive(Component)]
pub struct RichTextSection {
    styles: HashMap<TypeId, Box<dyn Style + Send + Sync + 'static>>,
    content: SectionText,
}
pub struct RichText {
    pub root_style: TextStyle,
    pub sections: Vec<RichTextSection>,
}
#[derive(Error, Debug)]
pub enum ParseError {}

impl RichText {
    /// Default rich text parser.
    pub fn parse(def: &str) -> Result<Self, ParseError> {
        todo!()
    }
    // TODO(text): consider RichText independent from entity, might control several
    pub fn 
}