//! Integrate [`RichText`] with bevy stuff.
// TODO(clean): this module should be renamed to something like "bevy_integration"
pub mod fetchers;
pub mod setter;
pub mod trackers;

use std::{any::TypeId, fmt};

use bevy::prelude::*;
use thiserror::Error;

use super::{modifiers, Bindings, Content, Modify, ModifyBox, RichText, TypeBindings};

/// Turn any type into a [modifier](ModifyBox).
///
/// Used in [`RichTextData::set`] and [`RichTextData::set_typed`].
pub trait IntoModify {
    fn into_modify(self) -> ModifyBox;
}
impl IntoModify for Color {
    fn into_modify(self) -> ModifyBox {
        Box::new(modifiers::Color(self))
    }
}
impl<T: Modify + Send + Sync + 'static> IntoModify for T {
    fn into_modify(self) -> ModifyBox {
        Box::new(self)
    }
}

// TODO(err): proper naming of types
#[derive(Error, Debug)]
pub enum BindingError {
    #[error("Innexisting name binding \"{key}\" for given type {id:?}")]
    NoKey { key: &'static str, id: TypeId },
    #[error("Innexisting type: \"{key:?}\"")]
    NoType { key: TypeId },
}
pub type BindingResult = Result<(), BindingError>;

/// Bindings read by all [`RichText`]s.
///
/// Unlike [`RichTextData`], this doesn't support type binding, because they
/// would necessarily be shared between all
#[derive(Resource, Default)]
pub struct GlobalRichTextBindings {
    bindings: Bindings,
    has_changed: bool,
}
impl GlobalRichTextBindings {
    fn insert_binding(&mut self, key: &'static str, value: ModifyBox) {
        self.has_changed = true;
        self.bindings.insert(key, value);
    }
    /// Set a named modifier binding.
    ///
    /// Unlike [`RichTextData`] this doesn't check that the key exists or that
    /// `value` is of the right type.
    pub fn set(&mut self, key: &'static str, value: impl IntoModify) {
        self.insert_binding(key, value.into_modify())
    }
    /// Set a named content binding.
    ///
    /// Unlike [`RichTextData`] this doesn't check that the key exists or that
    /// `value` is of the right type.
    pub fn set_content(&mut self, key: &'static str, value: &impl fmt::Display) {
        self.insert_binding(key, Box::new(Content::from(value)))
    }
}
#[derive(Component)]
pub struct RichTextData {
    text: RichText,
    bindings: Bindings,
    type_bindings: TypeBindings,
    base_style: TextStyle,
    // TODO(perf): better caching
    has_changed: bool,
}
impl fmt::Debug for RichTextData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RichTextData")
            .field("text", &self.text)
            .field("bindings", &self.bindings.values().collect::<Vec<_>>())
            .field(
                "type_bindings",
                &format!("{} TypeIds", self.type_bindings.len()),
            )
            .field("base_style", &self.base_style)
            .finish()
    }
}
impl RichTextData {
    fn insert_type_binding_checked(&mut self, key: TypeId, value: ModifyBox) -> BindingResult {
        if self.text.has_type_binding(key) {
            self.has_changed = true;
            self.type_bindings.insert(key, value);
            Ok(())
        } else {
            Err(BindingError::NoType { key })
        }
    }
    fn insert_binding_checked(
        &mut self,
        key: &'static str,
        id: TypeId,
        value: ModifyBox,
    ) -> BindingResult {
        if self.text.has_binding(key, id) {
            self.has_changed = true;
            self.bindings.insert(key, value);
            Ok(())
        } else {
            Err(BindingError::NoKey { key, id })
        }
    }
    pub fn set(&mut self, key: &'static str, value: impl IntoModify) -> BindingResult {
        let modifier = value.into_modify();
        let type_id = modifier.as_any().type_id();
        self.insert_binding_checked(key, type_id, modifier)
    }
    pub fn set_typed(&mut self, value: impl IntoModify) -> BindingResult {
        let modifier = value.into_modify();
        let type_id = modifier.as_any().type_id();
        self.insert_type_binding_checked(type_id, modifier)
    }
    pub fn set_content(
        &mut self,
        key: Option<&'static str>,
        value: &impl fmt::Display,
    ) -> BindingResult {
        let value = Box::new(Content::from(value));
        let id = TypeId::of::<Content>();
        match key {
            Some(key) => self.insert_binding_checked(key, id, value),
            None => self.insert_type_binding_checked(id, value),
        }
    }
}

// TODO(feat): generalize so that it works with Text2dBundle as well
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
            type_bindings: TypeBindings::default(),
            base_style,
            has_changed: true,
        };
        let mut text = TextBundle::default();
        let ctx = super::Context {
            bindings: None,
            type_bindings: None,
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
