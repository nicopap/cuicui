use std::mem;

use bevy::{ecs::system::SystemState, prelude::*, text::BreakLineOn};

use crate::{richtext, richtext::GetFont, richtext::RichTextData, track::Hook, Hooks};

use super::WorldBindings;

#[derive(Component)]
pub struct MakeRichText {
    pub text: TextSection,
    pub alignment: TextAlignment,
    pub line_break: BreakLineOn,
    pub format_string: String,
}
impl MakeRichText {
    /// Drain all fields from a `&mut Self` to get an owned value.
    fn take(&mut self) -> Self {
        Self {
            text: mem::take(&mut self.text),
            alignment: self.alignment,
            line_break: self.line_break,
            format_string: mem::take(&mut self.format_string),
        }
    }
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
                text: default(),
                alignment: TextAlignment::Left,
                line_break: BreakLineOn::WordBoundary,
                format_string: format_string.into(),
            },
            text_bundle: default(),
        }
    }
    pub fn with_text_style(mut self, style: TextStyle) -> Self {
        self.make_richtext.text.style = style;
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

/// Create and insert [`RichText`] on [`MakeRichText`] entities, updating [`WorldBindings`] and [`Hooks`].
///
/// This is an exclusive system, as it requires access to the [`World`] to generate
/// the [`Hook`]s specified in the format string.
pub fn mk_richtext(
    world: &mut World,
    mut to_make: Local<QueryState<(Entity, &mut MakeRichText)>>,
    mut cache: Local<SystemState<(Commands, ResMut<WorldBindings>, Res<Assets<Font>>)>>,
) {
    // The `format_string` are field of `MakeRichText`, components of the ECS.
    // we use `MakeRichText::take` to extract them from the ECS, and own them
    // in this system in `to_make`.
    let to_make: Vec<_> = to_make
        .iter_mut(world)
        .map(|(e, mut r)| (e, r.take()))
        .collect();

    // The `parse::Hook`s returned by `richtext::mk`
    // have a lifetime dependent on the `format_string` used.
    //
    // parse::Hook's reference here points to String within MakeRichText in
    // the `to_make` variable.
    let mut new_hooks: Vec<crate::parse::Hook<'_>> = Vec::new();

    // Furthermore, `richtext::mk` needs mutable access to WorldBindings and
    // immutable to Assets<Font>, so we use the SystemState to extract them.
    let (mut cmds, mut world_bindings, fonts) = cache.get_mut(world);
    // TODO(perf): batch commands update.
    for (entity, to_make) in to_make.iter() {
        let MakeRichText { text, alignment, line_break, format_string } = to_make;
        let b = &mut world_bindings;
        let fonts = GetFont::new(&fonts);
        match richtext::mk(b, text, fonts, *alignment, *line_break, format_string) {
            Ok((default_text, text, mut pulls)) => {
                new_hooks.append(&mut pulls);

                let richtext_data = RichTextData::new(text, to_make.text.clone());
                cmds.entity(*entity)
                    .insert((richtext_data, default_text))
                    .remove::<MakeRichText>();
            }
            Err(err) => {
                error!("Error when building a richtext: {err}");
            }
        }
    }
    cache.apply(world);

    // To convert the parse::Hook into an actual track::Hook that goes into track::Hooks,
    // we need excluisve world access.
    world.resource_scope(|world, mut hooks: Mut<Hooks>| {
        world.resource_scope(|world, mut bindings: Mut<WorldBindings>| {
            new_hooks.iter().for_each(|hook| {
                if let Some(hook) = Hook::from_parsed(*hook, &mut bindings, world) {
                    hooks.extend(Some(hook));
                } else {
                    error!("A tracker failed to be loaded");
                }
            });
        });
    });
}
