//! Integrate [`RichText`] with bevy stuff.

use std::{any::TypeId, fmt};

use bevy::{asset::HandleId, prelude::*};
use thiserror::Error;

use crate::{
    modifiers::Content,
    modify::{self, Bindings, TypeBindings},
    track::{update_tracked_components, update_tracked_resources},
    IntoModify, ModifyBox, ResTrackers, RichText,
};

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
pub struct WorldBindings {
    bindings: Bindings,
    has_changed: bool,
}
impl WorldBindings {
    fn insert(&mut self, key: &'static str, value: ModifyBox) {
        self.has_changed = true;
        self.bindings.insert(key, value);
    }
    /// Set a named modifier binding.
    ///
    /// Unlike [`RichTextData`] this doesn't check that the key exists or that
    /// `value` is of the right type.
    pub fn set(&mut self, key: &'static str, value: impl IntoModify) {
        self.insert(key, value.into_modify())
    }
    /// Set a named content binding.
    ///
    /// Unlike [`RichTextData`] this doesn't check that the key exists or that
    /// `value` is of the right type.
    pub fn set_content(&mut self, key: &'static str, value: &impl fmt::Display) {
        self.insert(key, Box::new(Content::from(value)))
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
        let ctx = modify::Context {
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

pub fn update_text(
    mut query: Query<(&mut RichTextData, &mut Text)>,
    mut global_context: ResMut<WorldBindings>,
    fonts: Res<Assets<Font>>,
) {
    for (mut rich, mut text) in &mut query {
        if global_context.has_changed {
            let ctx = modify::Context {
                bindings: Some(&global_context.bindings),
                type_bindings: None,
                parent_style: &rich.base_style,
                fonts: &|name| Some(fonts.get_handle(HandleId::from(name))),
            };
            rich.text.update(&mut text, &ctx);
            global_context.has_changed = false;
        }
        if rich.has_changed {
            let ctx = modify::Context {
                bindings: Some(&rich.bindings),
                type_bindings: Some(&rich.type_bindings),
                parent_style: &rich.base_style,
                fonts: &|name| Some(fonts.get_handle(HandleId::from(name))),
            };
            rich.text.update(&mut text, &ctx);
            rich.has_changed = false;
        }
    }
}

/// Plugin to update bevy [`Text`] component based on [`GlobalRichTextBindings`]
/// and [`RichTextData`] content.
pub struct RichTextPlugin;
impl Plugin for RichTextPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldBindings>()
            .add_system(
                update_tracked_resources
                    .in_base_set(CoreSet::PostUpdate)
                    .run_if(resource_exists::<ResTrackers>()),
            )
            .add_system(update_tracked_components.in_base_set(CoreSet::PostUpdate))
            .add_system(update_text.in_base_set(CoreSet::PostUpdate));
    }
}