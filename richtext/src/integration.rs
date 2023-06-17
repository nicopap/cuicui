use std::{fmt, fmt::Write, marker::PhantomData};

#[cfg(feature = "richtext")]
use bevy::text::{Text, TextSection};
use bevy::{
    ecs::{
        query::WorldQuery,
        system::{lifetimeless::SRes, EntityCommands, SystemParam, SystemParamItem},
    },
    prelude::*,
    text::{BreakLineOn, Font, TextAlignment},
};
use bevy_fab::trait_extensions::AppStylesExtension;
use bevy_fab::{BevyModify, FabPlugin, LocalBindings, ParseFormatString};
use fab_parse::{Split, Styleable};

#[cfg(feature = "cresustext")]
use crate::modifiers::ModifierQuery;
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

#[cfg(not(feature = "cresustext"))]
#[derive(SystemParam)]
pub struct WorldBindings<'w, 's> {
    bindings: Res<'w, bevy_fab::WorldBindings<Modifier>>,
    context: Res<'w, Assets<Font>>,
    _p: PhantomData<&'s ()>,
}
#[derive(SystemParam)]
pub struct WorldBindingsMut<'w, 's> {
    bindings: ResMut<'w, bevy_fab::WorldBindings<Modifier>>,
    #[cfg(feature = "cresustext")]
    items: Query<'w, 's, ModifierQuery>,
    #[cfg(feature = "cresustext")]
    context: Res<'w, Assets<Font>>,
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
    #[cfg(feature = "richtext")]
    text: &'static mut Text,
    #[cfg(feature = "cresustext")]
    children: Option<&'static Children>,
}
impl RichTextItem<'_> {
    /// Update `to_update` with updated values from `world` and `self`-local bindings.
    ///
    /// Only the relevant sections of `to_update` are updated. The change trackers
    /// are then reset.
    #[cfg(feature = "richtext")]
    pub fn update(&mut self, world: &WorldBindings) {
        let fonts = GetFont::new(&world.context);
        self.inner.update(&mut self.text, &world.bindings, &fonts);
    }
    /// Update `to_update` with updated values from `world` and `self`-local bindings.
    ///
    /// Only the relevant sections of `to_update` are updated. The change trackers
    /// are then reset.
    #[cfg(feature = "cresustext")]
    pub fn update(&mut self, world: WorldBindingsMut) {
        let fonts = GetFont::new(&world.context);
        let mut items = bevy_fab::Items::new(self.children, world.items);
        self.inner.update(&mut items, &world.bindings, &fonts);
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
        #[cfg(feature = "richtext")]
        let default_text = default();
        #[cfg(feature = "cresustext")]
        let default_text = {
            let default_section = TextSection::default();
            let mut default_text = Text::default();
            default_text.sections.push(default_section);
            (default(), default_text)
        };
        let inner = ParseFormatString::new(format_string.into(), default_text, default());
        MakeRichText { inner, text_bundle: default() }
    }
    pub fn with_text_style(mut self, style: TextStyle) -> Self {
        #[cfg(feature = "richtext")]
        {
            self.inner.default_item.style = style;
        }
        #[cfg(feature = "cresustext")]
        {
            self.inner.default_item.1.sections[0].style = style;
        }
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

    fn context<'a>(param: &'a SystemParamItem<Self::Param>) -> Self::Context<'a> {
        GetFont::new(param)
    }

    #[cfg(feature = "richtext")]
    fn spawn_items(extra: &TextGlobalStyle, sections: Vec<TextSection>, cmds: &mut EntityCommands) {
        cmds.insert(Text {
            sections,
            alignment: extra.alignment,
            linebreak_behaviour: extra.linebreak_behaviour,
        });
    }
    #[cfg(feature = "cresustext")]
    fn spawn_items(
        extra: &TextGlobalStyle,
        sections: Vec<(Transform, Text)>,
        cmds: &mut EntityCommands,
    ) {
        cmds.insert(NodeBundle::default());
        cmds.with_children(|cmds| {
            sections.into_iter().for_each(|(transform, text)| {
                cmds.spawn(TextBundle {
                    text: Text {
                        alignment: extra.alignment,
                        linebreak_behaviour: extra.linebreak_behaviour,
                        ..text
                    },
                    transform,
                    ..default()
                });
            });
        });
    }
    fn add_update_system(app: &mut App) {
        use bevy::prelude::CoreSet::PostUpdate;
        #[cfg(feature = "richtext")]
        {
            app.add_system(bevy_fab::update_component_items::<Self>.in_base_set(PostUpdate));
        }
        #[cfg(feature = "cresustext")]
        {
            app.add_system(
                bevy_fab::update_children_system::<ModifierQuery, Self>.in_base_set(PostUpdate),
            );
        }
    }
}

fn default_styles(tree: Styleable<Modifier>) -> Styleable<Modifier> {
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

/// Plugin to add to get `RichText` stuff working, it wouldn't otherwise, you silly goose.
pub struct RichTextPlugin {
    fab: FabPlugin<Modifier>,
    default_styles: bool,
}
impl RichTextPlugin {
    /// Initialize the `RichTextPlugin` with given style.
    ///
    /// See [`crate::Styles`] documentation for a detailed breakdown on how to use this
    /// to its full potential.
    pub fn no_default_styles() -> Self {
        RichTextPlugin { fab: FabPlugin::new(), default_styles: false }
    }
    pub fn new() -> Self {
        RichTextPlugin { fab: FabPlugin::new(), default_styles: true }
    }
}
impl Default for RichTextPlugin {
    fn default() -> Self {
        RichTextPlugin::new()
    }
}
impl Plugin for RichTextPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        self.fab.build(app);
        if self.default_styles {
            app.add_style(default_styles);
        }
    }
}
