//! Rich text and reactive text update for bevy.
//!
//! # Usage
//!
//! - [ ] Add [`RichTextPlugin`] to your app
//! - [ ] Create a [`RichText`] using [`RichTextBundle`] and add it to your UI.
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
//!     track, MakeRichTextBundle, IntoModify, ModifyBox,
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
//!     commands.spawn(
//!         MakeRichTextBundle::new(
//!             "{Color:{DeathLineColor}|Death count: {DeathCount}}\n\
//!          slider1 value: {slider1}\n\
//!          slider2 debug text: {slider2}",
//!         )
//!         .with_text_style(TextStyle {
//!             font: asset_server.load("fonts/FiraSans-Bold.ttf"),
//!             ..default()
//!         }),
//!     );
//! }
//! ```
//!
//! [the README]: https://github.com/nicopap/cuicui/tree/main/richtext
//! [`Resource`]: bevy::prelude::Resource

// mod hlist_madness;
mod binding;
pub mod change_text;
pub mod modifiers;
pub mod modify;
mod parse;
mod plugin;
mod richtext;
pub mod show;
pub mod track;

pub use binding::{BindingsView, LocalBindings};
pub use modify::{AnyError, IntoModify, Modify, ModifyBox};
pub use plugin::{
    make_rich, MakeRichText, MakeRichTextBundle, RichTextData, RichTextPlugin, WorldBindings,
};
pub use richtext::{RichText, RichTextBuilder};
pub use track::{ResTrackers, ResourceTrackerExt, Tracked};
