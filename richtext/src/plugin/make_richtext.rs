use std::mem;

use bevy::{asset::HandleId, prelude::*, text::BreakLineOn};

use crate::{modify, ResTrackers, RichTextData};

use super::WorldBindings;

#[derive(Component)]
pub struct MakeRichText {
    pub style: TextStyle,
    pub alignment: TextAlignment,
    pub linebreak_behaviour: BreakLineOn,
    pub format_string: String,
}

#[derive(Bundle)]
pub struct MakeRichTextBundle {
    pub text_bundle: TextBundle,
    pub make_richtext: MakeRichText,
}
/// Implementation of [`TextBundle`] delegate methods (ie: just pass the
/// call to the `text` field.
impl MakeRichTextBundle {
    pub fn new(format_string: impl Into<String>) -> Self {
        MakeRichTextBundle {
            make_richtext: MakeRichText {
                style: default(),
                alignment: TextAlignment::Left,
                linebreak_behaviour: BreakLineOn::WordBoundary,
                format_string: format_string.into(),
            },
            text_bundle: default(),
        }
    }
    pub fn with_text_style(mut self, style: TextStyle) -> Self {
        self.make_richtext.style = style;
        self
    }
    /// Returns this [`MakeRichTextBundle`] with a new [`TextAlignment`] on [`Text`].
    pub const fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.make_richtext.alignment = alignment;
        self
    }

    /// Returns this [`MakeRichTextBundle`] with a new [`Style`].
    pub fn with_style(mut self, style: Style) -> Self {
        self.text_bundle.style = style;
        self
    }

    /// Returns this [`MakeRichTextBundle`] with a new [`BackgroundColor`].
    pub const fn with_background_color(mut self, color: Color) -> Self {
        self.text_bundle.background_color = BackgroundColor(color);
        self
    }
}

pub fn make_rich(
    mut cmds: Commands,
    mut world_bindings: ResMut<WorldBindings>,
    mut res_trackers: ResMut<ResTrackers>,
    fonts: Res<Assets<Font>>,
    mut awaiting_fortune: Query<(Entity, &mut MakeRichText)>,
) {
    // TODO(perf): batch commands update.
    for (entity, mut make_rich) in &mut awaiting_fortune {
        let MakeRichText {
            style,
            alignment,
            linebreak_behaviour,
            format_string,
        } = &mut *make_rich;
        let format_string = mem::take(format_string);
        let builder = world_bindings.0.richtext_builder(format_string);
        match builder.build() {
            Ok((text, mut trackers)) => {
                res_trackers.extend(trackers.drain(..));

                let ctx = modify::Context {
                    bindings: world_bindings.0.view(),
                    parent_style: &*style,
                    fonts: &|name| Some(fonts.get_handle(HandleId::from(name))),
                };
                let mut default_text = text.default_text(&ctx);
                default_text.alignment = *alignment;
                default_text.linebreak_behaviour = *linebreak_behaviour;

                let richtext_data = RichTextData {
                    text,
                    base_style: mem::take(style),
                    bindings: default(),
                };
                cmds.entity(entity)
                    .insert((richtext_data, default_text))
                    .remove::<MakeRichText>();
            }
            Err(err) => {
                error!("Error when building a richtext: {err}");
            }
        }
    }
}
