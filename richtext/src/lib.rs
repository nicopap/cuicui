//! Rich text and reactive text update for bevy.
//!
//! # Usage
//!
//! - [ ] Add [`RichTextPlugin`] to your app
//! - [ ] Create a [`RichText`] using [`RichTextBundle`] and add it to your UI.
//!       See [`RichText::parse`] for a primer on the format string syntax.
//! - [ ] Update the [`RichText`]'s context. There is actually multiple approaches:
//!     - Use the [`track!`] macro to make [`RichText`] binding's follow the value
//!       of components you added to the ECS.
//!     - Use the [`ResourceTrackerExt`] methods to track some [`Resource`]s.
//!     - Manually use the [`RichTextData::set`],
//!       [`set_typed`](RichTextData::set_typed) or [`set_content`](RichTextData::set_content)
//!       to update a specific [`RichText`] context.
//!     - Update [`WorldBindings`] to update the context of all the [`RichText`]s
//!       present in the ECS.
//!
//! # Example
//!
//! Following is a short example.
//! Please follow the links in the previous sections for usage details.
//!
//! You may also be interested in [the README] for a more in-depth and "flat" presentation.
//!
//! ```rust
//! # use std::fmt;
//! use bevy::prelude::*;
//! use cuicui_richtext::{
//!     track, RichTextBundle, IntoModify, ModifyBox,
//!     modifiers, ResourceTrackerExt,
//! };
//! # #[derive(Component, Default)]
//! # struct MaxValue(f32);
//! #
//! # #[derive(Component, Default)]
//! # struct MinValue(f32);
//! #
//! #[derive(Component, Default, Reflect, Debug)]
//! struct Slider(f32);
//!
//! impl fmt::Display for Slider {
//!     // ...
//! #    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//! #        write!(f, "{:0.3}", self.0)
//! #    }
//! }
//! #[derive(Bundle, Default)]
//! struct RangeBundle { // ...
//! #    max: MaxValue,
//! #    min: MinValue,
//! }
//! #[derive(Bundle, Default)]
//! struct SliderBundle { // ...
//! #    max: MaxValue,
//! #    min: MinValue,
//!     slider: Slider,
//! }
//! #[derive(Resource, Clone, Copy, Reflect, Default)]
//! struct DeathCount(u32);
//!
//! impl fmt::Display for DeathCount {
//!     // ...
//! #    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//! #        write!(f, "{:}", self.0)
//! #    }
//! }
//! #[derive(Resource, Clone, Reflect)]
//! struct DeathLineColor(Color);
//!
//! impl IntoModify for DeathLineColor {
//!     fn into_modify(self) -> ModifyBox {
//!         Box::new(modifiers::Color(self.0))
//!     }
//! }
//! fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     let value = 3.41;
//!
//!     commands.spawn((
//!         RangeBundle { max: MaxValue(34.0), ..default() },
//!         track!(slider1, Slider(value)),
//!     ));
//!     commands
//!         .spawn(SliderBundle { max: MaxValue(34.0), ..default() })
//!         .insert(track!('d, slider2, Slider(value)));
//!
//!     commands.init_tracked_resource::<DeathCount>();
//!     commands.insert_modify_resource(DeathLineColor(Color::RED));
//!
//!     // Rich text will automatically be updated.
//!     commands.spawn(RichTextBundle::parse(
//!         "{color:$DeathLineColor|Death count: {DeathCount}}\n\
//!         slider1 value: {slider1}\n\
//!         slider2 debug text: {slider2}",
//!         TextStyle {
//!             font: asset_server.load("fonts/FiraSans-Bold.ttf"),
//!             ..default()
//!         },
//!     ));
//! }
//! ```
//!
//! [the README]: https://github.com/devildahu/bevy_mod_cuicui/tree/main/richtext

mod gold_hash;
pub mod modifiers;
pub mod modify;
mod parse;
mod plugin;
mod pull;
mod short_name;
mod show;
pub mod track;

use std::any::TypeId;

use bevy::prelude::{Text, TextSection};

use modifiers::Dynamic;

pub use modify::{IntoModify, Modifiers, Modify, ModifyBox};
pub use parse::Error as ParseError;
pub use plugin::{RichTextBundle, RichTextData, RichTextPlugin, WorldBindings};
pub use track::{ResTrackers, ResourceTrackerExt, Tracked};

// TODO(perf): See design_doc/richtext/better_section_impl.md
// TODO(perf): should have change tracking (might require internal mutability)
// to be precise and extremely limited about what we update.
// TODO(clean): should separate Content from other modifiers, since there is always
// exactly one per section (I kept it as Modifier because I can re-use Dynamic)
#[derive(PartialEq, Debug, Default)]
pub struct Section {
    modifiers: Modifiers,
}

#[derive(Debug)]
pub struct RichText {
    // TODO(perf): this might be improved, for example by storing a binding-> section
    // list so as to avoid iterating over all sections when updating
    pub sections: Vec<Section>,
}

impl RichText {
    fn any_section(&self, id: TypeId, f: impl Fn(Option<&Dynamic>) -> bool) -> bool {
        self.sections
            .iter()
            .flat_map(|mods| mods.modifiers.get(&id))
            .any(|modifier| f(modifier.as_any().downcast_ref()))
    }
    /// Check if a type binding exists for given type
    pub fn has_type_binding(&self, id: TypeId) -> bool {
        // TODO(perf): probably can do better.
        self.any_section(id, |modifier| matches!(modifier, Some(&Dynamic::ByType(_))))
    }

    /// Check if a named binding exists, and has the provided type.
    pub fn has_binding(&self, binding: &str, id: TypeId) -> bool {
        // TODO(perf): probably can do better.
        self.any_section(id, |modifier| {
            let Some(Dynamic::ByName(name)) = modifier else { return false; };
            &**name == binding
        })
    }

    /// Default cuicui rich text parser. Using a syntax inspired by rust's `format!` macro.
    ///
    /// See [rust doc](https://doc.rust-lang.org/stable/std/fmt/index.html).
    pub fn parse(input: &str) -> Result<Self, ParseError<'_>> {
        parse::rich_text(input)
    }
    // TODO(text): consider RichText independent from entity, might control several
    pub fn update(&self, to_update: &mut Text, ctx: &modify::Context) {
        to_update.sections.resize_with(self.sections.len(), || {
            TextSection::from_style(ctx.parent_style.clone())
        });

        let rich = self.sections.iter();
        let poor = to_update.sections.iter_mut();

        for (to_set, value) in poor.zip(rich) {
            for modifier in value.modifiers.values() {
                modifier.apply(ctx, to_set);
            }
        }
    }
}
