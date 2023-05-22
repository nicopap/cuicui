use std::fmt;

use bevy::{
    prelude::{Component, Handle},
    reflect::{Reflect, Typed},
    text::{BreakLineOn, Font, Text, TextAlignment, TextSection},
    utils::HashMap,
};
use enumset::{EnumSetType, __internal::EnumSetTypePrivate};
use fab::{
    binding,
    prefab::{Prefab, Tracked},
    resolve::Resolver,
};

use crate::{modifiers::ModifyBox, parse, parse::interpret, show, show::ShowBox, track::Tracker};

// TODO(clean): Cleanup API, only make pub opaque newtypes.

pub type GetFont<'a> = &'a dyn Fn(&str) -> Option<Handle<Font>>;

#[derive(EnumSetType, Debug, PartialOrd, Ord)]
pub enum Field {
    FontSize,
    Font,
    Color,
}
pub struct TrackedText(Tracked<TextPrefab>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextPrefab {}
impl Prefab for TextPrefab {
    type Modifiers = ModifyBox;
    type Item = TextSection;
    type Items = Vec<TextSection>;
}
#[derive(Debug)]
pub struct RichText(Resolver<TextPrefab, { (Field::BIT_WIDTH - 1) as usize }>);
impl RichText {
    pub fn update(
        &self,
        to_update: &mut Text,
        updates: &TrackedText,
        bindings: binding::View<TextPrefab>,
        ctx: &GetFont,
    ) {
        self.0
            .update(&mut to_update.sections, &updates.0, bindings, ctx);
    }
}

#[derive(Component)]
pub struct RichTextData {
    pub text: RichText,
    pub inner: TrackedText,
    pub bindings: binding::Local<TextPrefab>,
}
impl RichTextData {
    pub fn update(
        &mut self,
        to_update: &mut Text,
        world: &binding::World<TextPrefab>,
        ctx: GetFont,
    ) {
        let Self { text, bindings, inner } = self;

        let view = world.view_with_local(bindings).unwrap();
        text.update(to_update, inner, view, &ctx);
        inner.0.reset_updated();
        bindings.reset_changes();
    }
    pub fn new(text: RichText, inner: TextSection) -> Self {
        RichTextData {
            inner: TrackedText(Tracked::new(inner)),
            bindings: Default::default(),
            text,
        }
    }
    pub fn set(&mut self, binding_name: impl Into<String>, value: ModifyBox) {
        self.bindings.set(binding_name, value)
    }
    pub fn set_by_id(&mut self, id: binding::Id, value: ModifyBox) {
        self.bindings.set_by_id(id, value)
    }
}

pub struct RichTextBuilder<'a> {
    pub format_string: String,
    pub(crate) context: interpret::Context<'a>,

    pub base_section: TextSection,
    pub get_font: GetFont<'a>,
    pub alignment: TextAlignment,
    pub linebreak_behaviour: BreakLineOn,

    // TODO(perf): This sucks, the `FetchBox`, which we are using this for, is
    // calling itself the `ShowBox` impl. Instead of storing formatters, we should
    // directly construct the `FetchBox` when it is added
    // TODO(feat): This is actually unused.
    pub formatters: HashMap<&'static str, ShowBox>,
}
impl<'a> RichTextBuilder<'a> {
    /// Add a [formatter](crate::show::Show).
    pub fn fmt<I, O, F>(mut self, name: &'static str, convert: F) -> Self
    where
        I: Reflect + Typed,
        O: fmt::Display + 'static, // TODO(bug): shouldn't need this + 'static
        F: Clone + Send + Sync + Fn(&I) -> O + 'static,
    {
        self.formatters
            .insert(name, show::Convert::<I, O, F>::new(convert));
        self
    }
    pub fn build(self) -> anyhow::Result<(Text, RichText, Vec<Tracker>)> {
        let Self { format_string, mut context, base_section, .. } = self;
        let mut trackers = Vec::new();
        let modifiers = parse::richtext(&mut context, &format_string, &mut trackers)?;
        let default_section = base_section;

        let (rich_text, sections) = Resolver::new(modifiers, &default_section, &self.get_font);
        let text = Text {
            sections,
            alignment: self.alignment,
            linebreak_behaviour: self.linebreak_behaviour,
        };

        // debug!("Making RichText: {format_string:?}");
        // partial.print_bindings();
        Ok((text, RichText(rich_text), trackers))
    }
}
