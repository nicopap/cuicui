//! Integrate [`RichText`] with bevy stuff.

mod make_richtext;

use std::fmt;

use bevy::{asset::HandleId, prelude::*};
use fab::binding;

use crate::{
    modifiers::{self, Content, ModifyBox},
    richtext::{RichTextData, TextPrefab},
    track::{update_tracked_components, update_tracked_resources},
    ResTrackers,
};

pub use make_richtext::{make_rich, MakeRichText, MakeRichTextBundle};

/// Bindings read by all [`RichText`]s.
///
/// Unlike [`RichTextData`], this doesn't support type binding, because they
/// would necessarily be shared between all
#[derive(Resource, Default)]
pub struct WorldBindings(binding::World<TextPrefab>);
impl WorldBindings {
    /// Set a named modifier binding.
    ///
    /// Unlike [`RichTextData`] this doesn't check that the key exists or that
    /// `value` is of the right type.
    pub fn set(&mut self, key: &str, value: ModifyBox) {
        self.0.set_neq(key, value);
    }
    /// Set a named content binding.
    ///
    /// Unlike [`RichTextData`] this doesn't check that the key exists or that
    /// `value` is of the right type.
    pub fn set_content(&mut self, key: &str, value: &impl fmt::Display) {
        self.0.set_neq(key, Box::new(Content::from(value)));
    }
}

pub fn update_text(
    mut query: Query<(&mut RichTextData, &mut Text)>,
    world_bindings: ResMut<WorldBindings>,
    fonts: Res<Assets<Font>>,
) {
    for (mut rich, mut to_update) in &mut query {
        rich.update(&mut to_update, &world_bindings.0, &|name| {
            Some(fonts.get_handle(HandleId::from(name)))
        });
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
