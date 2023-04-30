use bevy::{asset::HandleId, ecs::query::WorldQuery, prelude::*};

use super::{super::Context, GlobalRichTextBindings, RichTextData};

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct RichTextSetter {
    pub rich: &'static mut RichTextData,
    pub text: &'static mut Text,
}
impl<'w> RichTextSetterItem<'w> {
    pub fn update(&mut self, fonts: &Assets<Font>) {
        if !self.rich.has_changed {
            return;
        }
        self.rich.has_changed = false;
        let ctx = Context {
            bindings: Some(&self.rich.bindings),
            type_bindings: Some(&self.rich.type_bindings),
            parent_style: &self.rich.base_style,
            fonts: &|name| Some(fonts.get_handle(HandleId::from(name))),
        };
        self.rich.text.update(&mut self.text, &ctx);
    }
}
pub fn update_text(
    mut query: Query<RichTextSetter>,
    mut global_context: ResMut<GlobalRichTextBindings>,
    fonts: Res<Assets<Font>>,
) {
    for mut text in &mut query {
        if global_context.has_changed {
            let ctx = Context {
                bindings: Some(&global_context.bindings),
                type_bindings: None,
                parent_style: &text.rich.base_style,
                fonts: &|name| Some(fonts.get_handle(HandleId::from(name))),
            };
            text.rich.text.update(&mut text.text, &ctx);
            global_context.has_changed = false;
        }
        text.update(&fonts);
        // dbg!(&text.text);
        // dbg!(&text.rich);
    }
}
