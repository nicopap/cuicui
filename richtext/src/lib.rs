//! Rich text and reactive text update for bevy.
//!
//! # Usage
//!
//! - [ ] Add [`RichTextPlugin`] to your app
//! - [ ] Create a [`RichText`] using [`MakeRichText`] and add it to your UI.
//! - [ ] Update the [`RichText`]'s context. There is actually multiple approaches:
//!     - Use the [`track!`] macro to make [`RichText`] binding's follow the value
//!       of components you added to the ECS.
//!     - Manually use the [`RichText::set`](integration::RichTextItem::set) or
//!       [`set_content`](integration::RichTextItem::set_content).
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
//! use cuicui_richtext::{track, MakeRichText, modifiers};
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
//! #[reflect(Resource)]
//! struct DeathCount(u32);
//!
//! #[derive(Resource, Clone, Default, Reflect)]
//! #[reflect(Resource)]
//! struct DeathLineColor(Color);
//!
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
//!     // Rich text will automatically be updated.
//!     commands.spawn(
//!         MakeRichText::new(
//!             "{Color:{Res.DeathLineColor.0}|Death count: {Res.DeathCount.0}}\n\
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

mod color;
mod integration;
pub mod modifiers;
mod track_macro;

pub use bevy_fab::{ReflectQueryable, TrackerBundle};
pub use integration::{
    MakeRichText, RichText, RichTextFetch, RichTextItem, RichTextPlugin, WorldBindings,
    WorldBindingsMut,
};
pub use modifiers::{GetFont, Modifier};
