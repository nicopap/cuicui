//! Integrate [`RichText`] with bevy stuff.

mod make_richtext;

use std::fmt::{self, Write};

use bevy::prelude::*;
use fab::binding;

use crate::{
    modifiers::TextModifiers,
    richtext::{GetFont, RichTextData, TextPrefab},
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
    pub fn set(&mut self, key: &str, value: TextModifiers) {
        self.0.set_neq(key, value);
    }
    /// Set a named content binding. This will mark it as changed.
    pub fn set_content(&mut self, key: &str, value: &impl fmt::Display) {
        if let Some(TextModifiers::Content { statik }) = self.0.get_mut(key) {
            let to_change = statik.to_mut();
            to_change.clear();
            write!(to_change, "{value}").unwrap();
        } else {
            let content = TextModifiers::content(value.to_string().into());
            self.0.set_neq(key, content);
        }
    }
}

pub fn update_text(
    mut query: Query<(&mut RichTextData, &mut Text)>,
    mut world_bindings: ResMut<WorldBindings>,
    fonts: Res<Assets<Font>>,
) {
    for (mut rich, mut to_update) in &mut query {
        rich.update(&mut to_update, &world_bindings.0, GetFont::new(&fonts));
        rich.bindings.reset_changes();
    }
    world_bindings.0.reset_changes();
}

/// Plugin to update bevy [`Text`] component based on [`WorldBindings`]
/// and [`RichTextData`] content.
pub struct RichTextPlugin;
impl Plugin for RichTextPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldBindings>()
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
