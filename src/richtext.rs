//! A rich text component managing bevy a single [`Text`].
//!
//! As with the rest of `bevy_cuicui`, you can either build one by parsing
mod color;
mod integrate;
pub mod modifiers;
mod parse;

use std::any::{Any, TypeId};
use std::fmt;

use bevy::prelude::Font as BevyFont;
use bevy::{prelude::*, utils::HashMap};

pub use integrate::{RichTextBundle, RichTextData, RichTextSetter, RichTextSetterItem};
pub use modifiers::{Color, Content, Dynamic, Font, RelSize};
pub use parse::Error as ParseError;

pub type ModifyBox = Box<dyn Modify + Send + Sync + 'static>;
pub type Modifiers = HashMap<TypeId, ModifyBox>;
// here we want to own the `dyn Modify`, we might potentially be able to "cache"
// it and modify it in place with new values.
pub type Bindings = HashMap<&'static str, ModifyBox>;

/// A [`TextSection`] modifier.
///
/// A [`TextSection`] may have an arbitary number of `Modify`s, modifying
/// the styling and content of a given section.
pub trait Modify {
    // TODO: error handling (ie missing dynamic modifer binding)
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()>;

    // those are workarounds to make tests in richtext/parse.rs work…
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }
    fn cmp(&self, _: &dyn Modify) -> bool {
        false
    }
    fn debug_show(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}
impl PartialEq for dyn Modify {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other)
    }
}
impl PartialEq for ModifyBox {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(&**other)
    }
}
impl fmt::Debug for dyn Modify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_show(f)
    }
}
impl fmt::Debug for ModifyBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_show(f)
    }
}

// TODO: more details, explain bidings.
/// The context used in [`Modify`].
#[derive(Clone, Copy)]
pub struct Context<'a, 'b> {
    pub bindings: Option<&'b Bindings>,
    pub parent_style: &'b TextStyle,
    // Note: we use a `&'a dyn` here instead of a type parameter because we intend
    // for `Context` to be a parameter for a trait object method. If `Context` had
    // a non-lifetime type parameter, it would require that method to have a type
    // parameter itself, but this would make it non-dispatchable: ie not available
    // on trait object.
    pub fonts: &'a dyn Fn(&str) -> Option<Handle<BevyFont>>,
}
impl<'a, 'b> Context<'a, 'b> {
    pub fn from_style(parent_style: &'b TextStyle) -> Self {
        Context { bindings: None, parent_style, fonts: &|_| None }
    }
}

// TODO(text): should have change tracking (might require internal mutability)
// to be precise and extremely limited about what we update.
#[derive(PartialEq, Debug)]
pub struct Section {
    modifiers: Modifiers,
}
#[derive(Debug)]
pub struct RichText {
    // TODO: this might be improved, for example by storing a binding-> section
    // list so as to avoid iterating over all sections when updating
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
