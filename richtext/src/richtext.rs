use std::{
    fmt::{self, Write},
    marker::PhantomData,
};

use bevy::{
    asset::HandleId,
    ecs::{
        query::WorldQuery,
        system::{lifetimeless::SRes, SystemParam, SystemParamItem},
    },
    prelude::*,
    text::{BreakLineOn, Font, Text, TextAlignment, TextSection},
};
use bevy_fab::{BevyPrefab, FabPlugin, ParseFormatString, PrefabLocal, PrefabWorld};
use enumset::__internal::EnumSetTypePrivate;
use fab::{prefab::Indexed, prefab::Prefab, prefab::PrefabContext};
use fab_parse::{Split, TransformedTree};

use crate::modifiers::{Modifier, ModifierField};

#[derive(Clone, Copy)]
pub struct TextGlobalStyle {
    alignment: TextAlignment,
    linebreak_behaviour: BreakLineOn,
}
impl Default for TextGlobalStyle {
    fn default() -> Self {
        TextGlobalStyle {
            alignment: TextAlignment::Left,
            linebreak_behaviour: BreakLineOn::WordBoundary,
        }
    }
}

#[derive(SystemParam)]
pub struct WorldBindings<'w, 's> {
    bindings: Res<'w, PrefabWorld<TextPrefab>>,
    context: Res<'w, Assets<Font>>,
    _p: PhantomData<&'s ()>,
}
#[derive(SystemParam)]
pub struct WorldBindingsMut<'w, 's> {
    bindings: ResMut<'w, PrefabWorld<TextPrefab>>,
    _p: PhantomData<&'s ()>,
}
impl<'w, 's> WorldBindingsMut<'w, 's> {
    pub fn set_content(&mut self, key: &str, value: &impl fmt::Display) {
        let Some(modifier) = self.bindings.0.get_mut(key) else {
            self.bindings.0.set(key, value.to_string().into());
            return;
        };
        modifier.write_fmt(format_args!("{value}")).unwrap();
    }
}
#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct RichText {
    inner: &'static mut PrefabLocal<TextPrefab, { (ModifierField::BIT_WIDTH - 1) as usize }>,
    text: &'static mut Text,
}
impl RichTextItem<'_> {
    /// Update `to_update` with updated values from `world` and `self`-local bindings.
    ///
    /// Only the relevant sections of `to_update` are updated. The change trackers
    /// are then reset.
    pub fn update(&mut self, world: &WorldBindings) {
        let fonts = GetFont::new(&world.context);
        self.inner.update(&mut self.text, &world.bindings, &fonts);
    }
    pub fn set(&mut self, key: &str, value: Modifier) {
        self.inner.bindings.set(key, value);
    }
    /// Set a named content binding. This will mark it as changed.
    pub fn set_content(&mut self, key: &str, value: &impl fmt::Display) {
        let Some(modifier) = self.inner.bindings.get_mut(key) else {
            self.inner.bindings.set(key, value.to_string().into());
            return;
        };
        modifier.write_fmt(format_args!("{value}")).unwrap();
    }
}
#[derive(Bundle)]
pub struct MakeRichText {
    inner: ParseFormatString<TextPrefab>,
    pub text_bundle: TextBundle,
}
impl MakeRichText {
    pub fn new(format_string: impl Into<String>) -> Self {
        let inner = ParseFormatString::new(format_string.into(), default(), default());
        MakeRichText { inner, text_bundle: default() }
    }
    pub fn with_text_style(mut self, style: TextStyle) -> Self {
        self.inner.default_item.style = style;
        self
    }
    /// Returns this [`MakeRichTextBundle`] with a new [`TextAlignment`] on [`Text`].
    pub fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        let extras = self.inner.items_extra.as_mut().unwrap();
        extras.alignment = alignment;
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

// TODO(clean): Make this private, only expose opaque wrappers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextPrefab {}
impl Prefab for TextPrefab {
    type Modify = Modifier;
    type Item = TextSection;
    type Items = Text;
}
impl Indexed<TextPrefab> for Text {
    fn get_mut(&mut self, index: usize) -> Option<&mut TextSection> {
        self.sections.as_mut_slice().get_mut(index)
    }
}
impl BevyPrefab for TextPrefab {
    type Param = SRes<Assets<Font>>;

    type ItemsCtorData = TextGlobalStyle;

    fn make_items(extra: &TextGlobalStyle, sections: Vec<TextSection>) -> Text {
        Text {
            sections,
            alignment: extra.alignment,
            linebreak_behaviour: extra.linebreak_behaviour,
        }
    }

    fn context<'a>(fonts: &'a SystemParamItem<Self::Param>) -> PrefabContext<'a, Self> {
        GetFont::new(fonts)
    }

    fn transform(tree: TransformedTree<'_, Self>) -> TransformedTree<'_, Self> {
        use Split::{ByChar, ByWord};

        let sin_curve = CardinalSpline::new_catmull_rom([1., 0., 1., 0., 1., 0.]);

        tree.acc_chop(ByChar, "Rainbow", |hue_offset: &mut f32, i, _| {
            Modifier::hue_offset(*hue_offset * i as f32)
        })
        .curve_chop(ByWord, "Sine", sin_curve.to_curve(), |ampl: &mut f32, t| {
            let size_change = (20.0 + t * *ampl).floor();
            Modifier::font_size(size_change)
        })
    }
}

#[derive(Default, Clone, Copy)]
pub struct GetFont<'a>(Option<&'a Assets<Font>>);
impl<'a> GetFont<'a> {
    pub fn new(assets: &'a Assets<Font>) -> Self {
        GetFont(Some(assets))
    }
    pub fn get(&self, name: &str) -> Option<Handle<Font>> {
        self.0.map(|a| a.get_handle(HandleId::from(name)))
    }
}

pub struct RichTextPlugin(FabPlugin<TextPrefab, { (ModifierField::BIT_WIDTH - 1) as usize }>);
impl RichTextPlugin {
    pub fn new() -> Self {
        RichTextPlugin(FabPlugin::new())
    }
}
impl Default for RichTextPlugin {
    fn default() -> Self {
        RichTextPlugin::new()
    }
}
impl Plugin for RichTextPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        self.0.build(app)
    }
}
