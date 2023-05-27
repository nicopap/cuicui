//! Integrate [`RichText`] with bevy stuff.

mod make_richtext;

use bevy::prelude::*;

use crate::{
    richtext::{GetFont, RichTextData, WorldBindings},
    track::{update_hooked, update_tracked_components},
    Hooks,
};

pub use make_richtext::{mk_richtext, MakeRichText, MakeRichTextBundle};

pub fn update_text(
    mut query: Query<(&mut RichTextData, &mut Text)>,
    mut world_bindings: ResMut<WorldBindings>,
    fonts: Res<Assets<Font>>,
) {
    for (mut rich, mut to_update) in &mut query {
        rich.update(&mut to_update, &world_bindings, GetFont::new(&fonts));
    }
    world_bindings.0.reset_changes();
}

/// Plugin to update bevy [`Text`] component based on [`WorldBindings`]
/// and [`RichTextData`] content.
pub struct RichTextPlugin;
impl Plugin for RichTextPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldBindings>()
            .init_resource::<Hooks>()
            .add_system(
                update_hooked
                    .in_base_set(CoreSet::PostUpdate)
                    .run_if(resource_exists::<Hooks>()),
            )
            .add_system(update_tracked_components.in_base_set(CoreSet::PostUpdate))
            .add_system(
                mk_richtext
                    .in_base_set(CoreSet::PostUpdate)
                    .before(update_text)
                    .run_if(resource_exists::<Hooks>()),
            )
            .add_system(update_text.in_base_set(CoreSet::PostUpdate));
    }
}
