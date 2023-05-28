use std::fmt::{self, Write};

use bevy::{
    asset::HandleId,
    prelude::{Assets, Component, Handle, Resource},
    text::{BreakLineOn, Font, Text, TextAlignment, TextSection},
};
use enumset::__internal::EnumSetTypePrivate;
use fab::{binding, prefab::Changing, prefab::Prefab, resolve::Resolver};

use crate::{modifiers::Modifier, modifiers::ModifierField, parse};

// TODO(clean): Make this private, only expose opaque wrappers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextPrefab {}
impl Prefab for TextPrefab {
    type Modify = Modifier;
    type Item = TextSection;
    type Items = Vec<TextSection>;
}

#[derive(Default)]
pub struct GetFont<'a>(Option<&'a Assets<Font>>);
impl<'a> GetFont<'a> {
    pub fn new(assets: &'a Assets<Font>) -> Self {
        GetFont(Some(assets))
    }
    pub fn get(&self, name: &str) -> Option<Handle<Font>> {
        self.0.map(|a| a.get_handle(HandleId::from(name)))
    }
}

/// Bindings read by all [`RichText`]s.
///
/// Unlike [`RichTextData`], this doesn't support type binding, because they
/// would necessarily be shared between all
#[derive(Resource, Default)]
pub struct WorldBindings(pub(crate) binding::World<TextPrefab>);
impl WorldBindings {
    /// Set a named modifier binding.
    pub fn set(&mut self, key: &str, value: Modifier) {
        self.0.set_neq(key, value);
    }
    /// Set a named modifier binding.
    pub fn set_id(&mut self, id: binding::Id, value: Modifier) {
        self.0.set_id_neq(id, value);
    }
    /// Set a named content binding. This will mark it as changed.
    pub fn set_content(&mut self, key: &str, value: &impl fmt::Display) {
        if let Some(Modifier::Content { statik }) = self.0.get_mut(key) {
            let to_change = statik.to_mut();
            to_change.clear();
            write!(to_change, "{value}").unwrap();
        } else {
            let content = Modifier::content(value.to_string().into());
            self.0.set_neq(key, content);
        }
    }
}

#[derive(Debug)]
pub struct RichText(Resolver<TextPrefab, { (ModifierField::BIT_WIDTH - 1) as usize }>);

pub struct TrackedText(Changing<TextSection, Modifier>);

#[derive(Component)]
pub struct RichTextData {
    pub text: RichText,
    pub inner: TrackedText,
    pub bindings: binding::Local<TextPrefab>,
}
impl RichTextData {
    /// Update `to_update` with updated values from `world` and `self`-local bindings.
    ///
    /// Only the relevant sections of `to_update` are updated. The change trackers
    /// are then reset.
    pub fn update(&mut self, to_update: &mut Text, world: &WorldBindings, ctx: GetFont) {
        let Self { text, bindings, inner } = self;

        // TODO(clean): this code should be in cuicui_fab
        let view = world.0.view_with_local(bindings).unwrap();
        text.0.update(&mut to_update.sections, &inner.0, view, &ctx);
        inner.0.reset_updated();
        bindings.reset_changes();
    }
    pub fn new(text: RichText, inner: TextSection) -> Self {
        RichTextData {
            inner: TrackedText(Changing::new(inner)),
            bindings: Default::default(),
            text,
        }
    }
    /// Set a named content binding. This will mark it as changed.
    pub fn set_content(&mut self, key: &str, value: &impl fmt::Display) {
        if let Some(Modifier::Content { statik }) = self.bindings.get_mut(key) {
            let to_change = statik.to_mut();
            to_change.clear();
            write!(to_change, "{value}").unwrap();
        } else {
            let content = Modifier::content(value.to_string().into());
            self.bindings.set(key, content);
        }
    }
}

/// Create a [`RichText`] by parsing `format_string`.
///
/// Also returns the initial [`Text`] value and the parsed, but not interpreted,
/// [`Hook`]s defined in the format string.
///
/// This also registers in `bindings` the binding names used in the format string.
///
/// [`Hook`]: crate::track::Hook
pub(crate) fn mk<'fstr>(
    bindings: &mut WorldBindings,
    base_section: &TextSection,
    get_font: GetFont,
    alignment: TextAlignment,
    linebreak_behaviour: BreakLineOn,
    format_string: &'fstr str,
) -> anyhow::Result<(Text, RichText, Vec<parse::Hook<'fstr>>)> {
    let mut pulls = Vec::new();

    let modifiers = parse::richtext(bindings, format_string, &mut pulls)?;

    let (rich_text, sections) = Resolver::new(modifiers.into_iter(), base_section, &get_font);
    let text = Text { sections, alignment, linebreak_behaviour };

    Ok((text, RichText(rich_text), pulls))
}
