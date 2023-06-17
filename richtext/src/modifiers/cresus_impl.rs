use std::{borrow::Cow, ops::Deref};

use bevy::prelude::*;
use bevy_fab::Items;
use bevy_layout_offset::UiOffset;
use fab::{impl_modify, Modify};

use super::{GetFont, ModifyBox};

pub type ModifierQuery = (&'static mut UiOffset, &'static mut Text);
pub type ModifierItem<'a> = (&'a mut UiOffset, &'a mut Text);

#[derive(Component)]
pub struct Sections(pub Box<[Entity]>);

impl Deref for Sections {
    type Target = [Entity];
    fn deref(&self) -> &Self::Target {
        &self.0[..]
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
    type MakeItem = (UiOffset, Text);
    type Item<'a> = ModifierItem<'a>;
    type Items<'a, 'b, 'c> = Items<'a, 'b, 'c, Sections, ModifierQuery>;

    /// Set the font to provided `path`.
    #[modify(context(get_font), write(.1.sections[0].style.font))]
    pub fn font(path: &Cow<'static, str>, get_font: &GetFont) -> Handle<Font> {
        trace!("Apply =font=: {path:?}");
        get_font.get(path).unwrap_or_default()
    }
    /// Increase the font size relative to the current section.
    #[modify(read_write(.1.sections[0].style.font_size))]
    pub fn rel_size(relative_size: f32, font_size: &mut f32) {
        trace!("Apply :rel_size: {relative_size:?}");
        *font_size *= relative_size;
    }
    /// Set font size to `size`.
    #[modify(write(.1.sections[0].style.font_size))]
    pub fn font_size(size: f32) -> f32 {
        size
    }
    /// Set the color of the [`TextSection`] to `statik`.
    #[modify(write(.1.sections[0].style.color))]
    pub fn color(statik: Color) -> Color {
        trace!("Apply ~COLOR~: {statik:?}");
        statik
    }
    /// Offset the color's Hue by `offset`.
    #[modify(read_write(.1.sections[0].style.color))]
    pub fn hue_offset(offset: f32, color: &mut Color) {
        trace!("Apply ~HueOffset~: {offset:?}");
        let mut hsl = color.as_hsla_f32();
        hsl[0] = (hsl[0] + offset) % 360.0;
        *color = Color::hsla(hsl[0], hsl[1], hsl[2], hsl[3]);
    }
    /// Set the text content of the [`TextSection`] to `statik`.
    #[modify(write_mut(.1.sections[0].value))]
    pub fn content(statik: &Cow<'static, str>, value: &mut String) {
        trace!("Apply $CONTENT$: {statik:?}");
        value.clear();
        value.push_str(statik);
    }
    /// Use an arbitrary [`ModifyBox`] to modify this section.
    #[modify(dynamic_read_write(depends, changes, item), context(ctx))]
    pub fn dynamic(boxed: &ModifyBox, ctx: &GetFont, item: ModifierItem) {
        boxed.apply(ctx, item);
    }
}
