use std::borrow::Cow;

use bevy::prelude::*;
use fab::{impl_modify, modify::Indexed, Modify};

use super::{GetFont, ModifyBox};

impl Indexed<Modifier> for Text {
    fn get_mut(&mut self, index: usize) -> Option<&mut TextSection> {
        self.sections.as_mut_slice().get_mut(index)
    }
}

/// Operations on bevy [`TextSection`]s.
///
/// You can create your own operations. At the cost of storing them as a [`ModifyBox`]
/// and having to be careful about what you update. You create such a `Modifier`
/// using [`Modifier::Dynamic`].
#[impl_modify(cuicui_fab_path = fab, no_derive(Debug))]
#[derive(PartialEq)]
impl Modify for Modifier {
    type Context<'a> = GetFont<'a>;
    type Item<'a> = &'a mut TextSection;
    type MakeItem = TextSection;
    type Items<'a, 'b, 'c> = Text;

    /// Set the font to provided `path`.
    #[modify(context(get_font), write(.style.font))]
    pub fn font(path: &Cow<'static, str>, get_font: &GetFont) -> Handle<Font> {
        trace!("Apply =font=: {path:?}");
        get_font.get(path).unwrap_or_default()
    }
    /// Increase the font size relative to the current section.
    #[modify(read_write(.style.font_size))]
    pub fn rel_size(relative_size: f32, font_size: &mut f32) {
        trace!("Apply :rel_size: {relative_size:?}");
        *font_size *= relative_size;
    }
    /// Set font size to `size`.
    #[modify(write(.style.font_size))]
    pub fn font_size(size: f32) -> f32 {
        size
    }
    /// Set the color of the [`TextSection`] to `statik`.
    #[modify(write(.style.color))]
    pub fn color(statik: Color) -> Color {
        trace!("Apply ~COLOR~: {statik:?}");
        statik
    }
    /// Offset the color's Hue by `offset`.
    #[modify(read_write(.style.color))]
    pub fn hue_offset(offset: f32, color: &mut Color) {
        trace!("Apply ~HueOffset~: {offset:?}");
        let mut hsl = color.as_hsla_f32();
        hsl[0] = (hsl[0] + offset) % 360.0;
        *color = Color::hsla(hsl[0], hsl[1], hsl[2], hsl[3]);
    }
    /// Set the text content of the [`TextSection`] to `statik`.
    #[modify(write_mut(.value))]
    pub fn content(statik: &Cow<'static, str>, value: &mut String) {
        trace!("Apply $CONTENT$: {statik:?}");
        value.clear();
        value.push_str(statik);
    }
    /// Use an arbitrary [`ModifyBox`] to modify this section.
    #[modify(dynamic_read_write(depends, changes, item), context(ctx))]
    pub fn dynamic(boxed: &ModifyBox, ctx: &GetFont, item: &mut TextSection) {
        boxed.apply(ctx, item);
    }
}
