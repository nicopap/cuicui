//! Integrate [`RichText`] with bevy stuff.

mod make_richtext;

use std::fmt;

use bevy::{asset::HandleId, prelude::*};

use crate::{
    binding::{self, LocalBindings},
    modifiers::{self, Content},
    modify::{BindingId, Context},
    track::{update_tracked_components, update_tracked_resources},
    IntoModify, ResTrackers, RichText,
};

pub use make_richtext::{make_rich, MakeRichText, MakeRichTextBundle};

/// Bindings read by all [`RichText`]s.
///
/// Unlike [`RichTextData`], this doesn't support type binding, because they
/// would necessarily be shared between all
#[derive(Resource, Default)]
pub struct WorldBindings(binding::WorldBindings);
impl WorldBindings {
    /// Set a named modifier binding.
    ///
    /// Unlike [`RichTextData`] this doesn't check that the key exists or that
    /// `value` is of the right type.
    pub fn set(&mut self, key: &str, value: impl IntoModify) {
        self.0.set(key, value);
    }
    /// Set a named content binding.
    ///
    /// Unlike [`RichTextData`] this doesn't check that the key exists or that
    /// `value` is of the right type.
    pub fn set_content(&mut self, key: &str, value: &impl fmt::Display) {
        self.0.set(key, Content::from(value));
    }
}

#[derive(Component)]
pub struct RichTextData {
    text: RichText,
    bindings: LocalBindings,
    base_style: TextStyle,
}
impl RichTextData {
    pub fn set(&mut self, binding_name: impl Into<String>, value: impl IntoModify) {
        self.bindings.set(binding_name, value)
    }
    pub fn set_by_id(&mut self, id: BindingId, value: impl IntoModify) {
        self.bindings.set_by_id(id, value)
    }
}

pub fn update_text(
    mut query: Query<(&mut RichTextData, &mut Text)>,
    world_bindings: ResMut<WorldBindings>,
    fonts: Res<Assets<Font>>,
) {
    for (mut rich, mut to_update) in &mut query {
        let RichTextData { text, bindings, base_style } = &mut *rich;

        let view = world_bindings.0.view_with_local(bindings);
        let ctx = Context {
            bindings: view.unwrap(),
            parent_style: base_style,
            fonts: &|name| Some(fonts.get_handle(HandleId::from(name))),
        };
        // TODO(perf): only update when changes are detected
        text.update(&mut to_update, &ctx);
    }
}

/// Plugin to update bevy [`Text`] component based on [`WorldBindings`]
/// and [`RichTextData`] content.
pub struct RichTextPlugin;
impl Plugin for RichTextPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<modifiers::Content>()
            .register_type::<modifiers::RelSize>()
            .register_type::<modifiers::Font>()
            .register_type::<modifiers::Color>()
            .init_resource::<WorldBindings>()
            .init_resource::<ResTrackers>()
            .add_system(
                update_tracked_resources
                    .in_base_set(CoreSet::PostUpdate)
                    .run_if(resource_exists::<ResTrackers>()),
            )
            .add_system(update_tracked_components.in_base_set(CoreSet::PostUpdate))
            .add_system(
                make_rich
                    .in_base_set(CoreSet::PostUpdate)
                    .before(update_text)
                    .run_if(resource_exists::<ResTrackers>()),
            )
            .add_system(update_text.in_base_set(CoreSet::PostUpdate));
    }
}
