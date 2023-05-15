use bevy::{
    prelude::{Color, Handle},
    text::{Font, TextStyle},
};
use enumset::EnumSet;

use crate::modify::Change;

pub struct ChangeTextStyle {
    pub changes: EnumSet<Change>,
    pub inner: TextStyle,
}
impl ChangeTextStyle {
    pub fn set_font(&mut self, new_value: Handle<Font>) {
        if new_value != self.inner.font {
            self.changes |= Change::Font;
            self.inner.font = new_value;
        }
    }
    pub fn set_font_size(&mut self, new_value: f32) {
        if new_value != self.inner.font_size {
            self.changes |= Change::FontSize;
            self.inner.font_size = new_value;
        }
    }
    pub fn set_color(&mut self, new_value: Color) {
        if new_value != self.inner.color {
            self.changes |= Change::Color;
            self.inner.color = new_value;
        }
    }
    pub fn reset_changes(&mut self) {
        self.changes = EnumSet::EMPTY;
    }
    pub fn new(inner: TextStyle) -> ChangeTextStyle {
        ChangeTextStyle { changes: EnumSet::EMPTY, inner }
    }
}
