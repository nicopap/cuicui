//! Integrate [`RichText`] with bevy stuff.

use core::fmt;

use bevy::{asset::HandleId, ecs::query::WorldQuery, prelude::*};

use super::{modifiers, Bindings, Modify, RichText};

// TODO: move `Bindings` to a `Res` so as to avoid duplicating info
#[derive(Component)]
pub struct RichTextData {
    text: RichText,
    bindings: Bindings,
    base_style: TextStyle,
    // TODO: better caching
    change_list: Vec<&'static str>,
}
impl fmt::Debug for RichTextData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RichTextData")
            .field("text", &self.text)
            .field("bindings", &self.bindings.keys().collect::<Vec<_>>())
            .field("base_style", &self.base_style)
            .field("change_list", &self.change_list)
            .finish()
    }
}
impl RichTextData {
    pub fn add_content(&mut self, key: &'static str, value: &impl fmt::Display) {
        self.change_list.push(key);
        self.bindings
            .insert(key, Box::new(modifiers::Content(value.to_string())));
    }
    pub fn add_binding(&mut self, key: &'static str, value: impl Modify + Send + Sync + 'static) {
        self.change_list.push(key);
        self.bindings.insert(key, Box::new(value));
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct RichTextSetter {
    pub rich: &'static mut RichTextData,
    pub text: &'static mut Text,
}
impl<'w> RichTextSetterItem<'w> {
    pub fn update(&mut self, fonts: &Assets<Font>) {
        if self.rich.change_list.is_empty() {
            return;
        }
        // TODO: use change list content to limit modifications
        self.rich.change_list.clear();
        let ctx = super::Context {
            bindings: Some(&self.rich.bindings),
            parent_style: &self.rich.base_style,
            fonts: &|name| Some(fonts.get_handle(HandleId::from(name))),
        };
        self.rich.text.update(&mut self.text, &ctx);
    }
}
// TODO: generalize so that it works with Text2dBundle as well
#[derive(Bundle)]
pub struct RichTextBundle {
    #[bundle]
    pub text: TextBundle,
    pub data: RichTextData,
}
impl RichTextBundle {
    pub fn parse(input: &str, base_style: TextStyle) -> Self {
        Self::new(RichText::parse(input).unwrap(), base_style)
    }
    pub fn new(rich: RichText, base_style: TextStyle) -> Self {
        let data = RichTextData {
            text: rich,
            bindings: Bindings::new(),
            base_style,
            change_list: Vec::new(),
        };
        let mut text = TextBundle::default();
        let ctx = super::Context {
            bindings: None,
            parent_style: &data.base_style,
            fonts: &|_| None,
        };
        data.text.update(&mut text.text, &ctx);
        RichTextBundle { text, data }
    }
}
/// Implementation of [`TextBundle`] delegate methods (ie: just pass the
/// call to the `text` field.
impl RichTextBundle {
    /// Returns this [`RichTextBundle`] with a new [`TextAlignment`] on [`Text`].
    pub const fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.text.text.alignment = alignment;
        self
    }

    /// Returns this [`RichTextBundle`] with a new [`Style`].
    pub fn with_style(mut self, style: Style) -> Self {
        self.text.style = style;
        self
    }

    /// Returns this [`RichTextBundle`] with a new [`BackgroundColor`].
    pub const fn with_background_color(mut self, color: Color) -> Self {
        self.text.background_color = BackgroundColor(color);
        self
    }
}
