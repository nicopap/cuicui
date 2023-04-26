//! A rich text component managing bevy a single [`Text`].
//!
//! As with the rest of `bevy_cuicui`, you can either build one by parsing
mod color;
mod integrate;
mod modifiers;
mod parse;

use std::any::TypeId;

use bevy::prelude::Font as BevyFont;
use bevy::{prelude::*, utils::HashMap};

pub use modifiers::{Color, Content, Dynamic, Font, RelSize};
pub use parse::Error as ParseError;

pub type ModifierBox = Box<dyn Modify + Send + Sync + 'static>;
pub type Modifiers = HashMap<TypeId, ModifierBox>;
pub type Bindings<'a> = HashMap<&'static str, &'a dyn Modify>;

/// A [`TextSection`] modifier.
///
/// A [`TextSection`] may have an arbitary number of `Modify`s, modifying
/// the styling and content of a given section.
pub trait Modify {
    // TODO: error handling (ie missing dynamic modifer binding)
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()>;
}

/// The context used in [`ApplySection`].
///
/// TODO: more details, explain bidings.
pub struct Context<'a, 'b> {
    pub bindings: Bindings<'b>,
    pub parent_style: TextStyle,
    pub fonts: &'a Assets<BevyFont>,
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
    pub fn update(&self, to_update: &mut Text, ctx: &Context) {
        to_update.sections.resize_with(self.sections.len(), default);

        let rich = self.sections.iter();
        let poor = to_update.sections.iter_mut();

        for (to_set, value) in poor.zip(rich) {
            for modifier in value.modifiers.values() {
                modifier.apply(ctx, to_set);
            }
        }
    }
}
