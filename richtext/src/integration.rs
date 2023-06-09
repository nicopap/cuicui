use std::{fmt, fmt::Write, marker::PhantomData};

use bevy::{
    ecs::{
        query::WorldQuery,
        system::{lifetimeless::SRes, SystemParam, SystemParamItem},
    },
    prelude::*,
    text::{BreakLineOn, Font, Text, TextAlignment, TextSection},
};
use bevy_fab::{BevyModify, FabPlugin, LocalBindings, ParseFormatString, StyleFn, Styles};
use fab_parse::{Split, TransformedTree};

use crate::modifiers::{GetFont, Modifier};

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
    bindings: Res<'w, bevy_fab::WorldBindings<Modifier>>,
    context: Res<'w, Assets<Font>>,
    _p: PhantomData<&'s ()>,
}
#[derive(SystemParam)]
pub struct WorldBindingsMut<'w, 's> {
    bindings: ResMut<'w, bevy_fab::WorldBindings<Modifier>>,
    _p: PhantomData<&'s ()>,
}
impl<'w, 's> WorldBindingsMut<'w, 's> {
    pub fn set_content(&mut self, key: &str, value: &impl fmt::Display) {
        let Some(modifier) = self.bindings.bindings.get_mut(key) else {
            self.bindings.bindings.set(key, value.to_string().into());
            return;
        };
        modifier.set_content(format_args!("{value}"));
    }
}
#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct RichText {
    inner: &'static mut LocalBindings<Modifier>,
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
        modifier.set_content(format_args!("{value}"));
    }
}
#[derive(Bundle)]
pub struct MakeRichText {
    inner: ParseFormatString<Modifier>,
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
    /// Returns this [`MakeRichText`] with a new [`TextAlignment`] on [`Text`].
    pub fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        let extras = self.inner.items_extra.as_mut().unwrap();
        extras.alignment = alignment;
        self
    }

    /// Returns this [`MakeRichText`] with a new [`Style`].
    pub fn with_style(mut self, style: Style) -> Self {
        self.text_bundle.style = style;
        self
    }

    /// Returns this [`MakeRichText`] with a new [`BackgroundColor`].
    pub const fn with_background_color(mut self, color: Color) -> Self {
        self.text_bundle.background_color = BackgroundColor(color);
        self
    }
}

impl BevyModify for Modifier {
    type Param = SRes<Assets<Font>>;

    type ItemsCtorData = TextGlobalStyle;

    fn make_items(extra: &TextGlobalStyle, sections: Vec<TextSection>) -> Text {
        Text {
            sections,
            alignment: extra.alignment,
            linebreak_behaviour: extra.linebreak_behaviour,
        }
    }

    fn context<'a>(fonts: &'a SystemParamItem<Self::Param>) -> Self::Context<'a> {
        GetFont::new(fonts)
    }

    fn set_content(&mut self, s: fmt::Arguments) {
        if let Modifier::Content { statik } = self {
            let statik = statik.to_mut();
            statik.clear();
            let _ = statik.write_fmt(s);
        } else {
            *self = Modifier::content(s.to_string().into());
        }
    }

    fn init_content(s: fmt::Arguments) -> Self {
        Modifier::content(s.to_string().into())
    }
}

fn default_styles(tree: TransformedTree<Modifier>) -> TransformedTree<Modifier> {
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

pub struct RichTextPlugin(FabPlugin<Modifier>, Styles<Modifier>);
impl RichTextPlugin {
    /// Initialize the `RichTextPlugin` with given style.
    ///
    /// See [`Styles`] documentation for a detailed breakdown on how to use this
    /// to its full potential.
    pub fn with_styles(styles: StyleFn<Modifier>) -> Self {
        RichTextPlugin(FabPlugin::new(), Styles::new(styles))
    }
    pub fn new() -> Self {
        RichTextPlugin(FabPlugin::new(), Styles::new(default_styles))
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
