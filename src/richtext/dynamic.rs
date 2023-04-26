use bevy::{prelude::Font as BevyFont, prelude::*, utils::HashMap};

/// A [`TextSection`] modifier.
///
/// A [`TextSection`] may have an arbitary number of `TextMod`s, modifying
/// the styling and content of a given section.
pub trait TextMod {
    // TODO: error handling (ie missing dynamic modifer binding)
    fn apply(&self, ctx: &TextContext, text: &mut TextSection) -> Option<()>;
}

pub type Bindings<'a> = HashMap<&'static str, &'a dyn TextMod>;

/// The context used to update [`TextStyle`]s for given bevy [`Text`] sections.
pub struct TextContext<'a, 'b> {
    pub bindings: Bindings<'b>,
    pub parent_style: TextStyle,
    pub fonts: &'a Assets<BevyFont>,
}

pub enum Dyn<T> {
    Set(T),
    Ref { name: String },
}
impl<T: TextMod> TextMod for Dyn<T> {
    fn apply(&self, ctx: &TextContext, text: &mut TextSection) -> Option<()> {
        match self {
            Dyn::Ref { name } => ctx.bindings.get(name.as_str())?.apply(ctx, text),
            Dyn::Set(value) => value.apply(ctx, text),
        }
    }
}
impl TextMod for () {
    fn apply(&self, _: &TextContext, _: &mut TextSection) -> Option<()> {
        Some(())
    }
}
