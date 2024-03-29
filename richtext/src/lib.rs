//! Rich text and reactive text update for bevy.
//!
//! # Usage
//!
//! - [ ] Add [`RichTextPlugin`] to your app
//! - [ ] Create a [`RichText`] using [`MakeRichText`] and add it to your UI.
//! - [ ] Update the [`RichText`]'s context. There is actually multiple approaches:
//!     - Use source bindings in your rich text format string to read directly
//!       from the ECS component/resource values.
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
//! use cuicui_richtext::{MakeRichText, modifiers, ReflectQueryable};
//! # #[derive(Component, Reflect, Default)]
//! # #[reflect(Component, Queryable)]
//! # struct MaxValue(f32);
//! #
//! # #[derive(Component, Reflect, Default)]
//! # #[reflect(Component, Queryable)]
//! # struct MinValue(f32);
//! #
//! #[derive(Component, Default, Reflect, Debug)]
//! #[reflect(Component, Queryable)]
//! struct Slider(f32);
//!
//! #[derive(Component, Reflect, Default)]
//! #[reflect(Component, Queryable)]
//! struct Slider1;
//!
//! #[derive(Component, Reflect, Default)]
//! #[reflect(Component, Queryable)]
//! struct Slider2;
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
//!         Slider(value),
//!         Slider1,
//!     ));
//!     commands.spawn(SliderBundle {
//!         max: MaxValue(34.0),
//!         slider: Slider(value),
//!         ..default()
//!     }).insert(Slider2);
//!
//!     // Rich text will automatically be updated.
//!     commands.spawn(
//!         MakeRichText::new(
//!             "{Color:{Res(DeathLineColor).0}|Death count: {Res(DeathCount).0}}\n\
//!          slider1 value: {Marked(Slider1).Slider.0}\n\
//!          slider2 debug text: {Marked(Slider2).Slider:?}",
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

#[cfg(any(
    all(feature = "min_richtext", feature = "richtext"),
    all(feature = "min_richtext", feature = "cresustext"),
    all(feature = "richtext", feature = "cresustext"),
))]
compile_error!(
    "The features 'min_richtext', 'richtext' and 'cresustext' cannot \
    be enabled at the same time, please make sure to disable default features if \
    you are not using cresustext"
);

mod color;
mod integration;
pub mod modifiers;

/// See [`bevy_fab::UserFmt`] docs.
pub type UserFmt = bevy_fab::UserFmt<Modifier>;
/// See [`bevy_fab::Styles`] docs.
pub type Styles = bevy_fab::Styles<Modifier>;

pub use bevy_fab::{FmtSystem, IntoFmtSystem, ReflectQueryable};
pub use fab::binding::{Entry, Id};
#[cfg(not(feature = "cresustext"))]
pub use integration::WorldBindings;
pub use integration::{
    MakeRichText, RichText, RichTextFetch, RichTextItem, RichTextPlugin, WorldBindingsMut,
};
pub use modifiers::{GetFont, Modifier};

pub mod trait_extensions {
    use bevy::reflect::Reflect;
    use bevy_fab::{
        trait_extensions::{AppFormattersExtension, AppStylesExtension},
        FmtSystem, IntoFmtSystem,
    };
    use fab::binding::Entry;
    use fab_parse::Styleable;

    use crate::{Modifier, UserFmt};

    /// Explicit [`AppStylesExtension`] for this crate's [`Modifier`]
    pub trait AppTextStylesExtension: AppStylesExtension<Modifier> {
        /// Insert a new style before all others.
        fn overwrite_style<F>(&mut self, style: F) -> &mut Self
        where
            F: FnMut(Styleable<Modifier>) -> Styleable<Modifier> + Send + Sync + 'static,
        {
            AppStylesExtension::overwrite_style(self, style)
        }
        /// Add a new style after existing ones.
        fn add_style<F>(&mut self, style: F) -> &mut Self
        where
            F: FnMut(Styleable<Modifier>) -> Styleable<Modifier> + Send + Sync + 'static,
        {
            AppStylesExtension::add_style(self, style)
        }
    }
    impl<T: AppStylesExtension<Modifier>> AppTextStylesExtension for T {}

    /// Explicit [`AppFormattersExtension`] for this crate's [`Modifier`]
    pub trait AppTextFormattersExtension: AppFormattersExtension<Modifier> {
        /// Add a plain [`UserFmt`].
        fn add_user_fmt(&mut self, name: impl AsRef<str>, fmt: UserFmt) -> &mut Self {
            AppFormattersExtension::add_user_fmt(self, name, fmt)
        }

        /// Add a formatter that may READ (only) the world.
        fn add_sys_fmt<T: FmtSystem<Modifier>>(
            &mut self,
            name: impl AsRef<str>,
            fmt: impl IntoFmtSystem<Modifier, T>,
        ) -> &mut Self {
            AppFormattersExtension::add_sys_fmt(self, name, fmt)
        }

        /// Add a simple function formatter.
        fn add_dyn_fn_fmt(
            &mut self,
            name: impl AsRef<str>,
            fmt: impl Fn(&dyn Reflect, Entry<Modifier>) + Send + Sync + 'static,
        ) -> &mut Self {
            AppFormattersExtension::add_dyn_fn_fmt(self, name, fmt)
        }

        /// Add a simple function formatter.
        fn add_fn_fmt<T: Reflect>(
            &mut self,
            name: impl AsRef<str>,
            fmt: impl Fn(&T, Entry<Modifier>) + Send + Sync + 'static,
        ) -> &mut Self {
            AppFormattersExtension::add_fn_fmt(self, name, fmt)
        }
    }
    impl<T: AppFormattersExtension<Modifier>> AppTextFormattersExtension for T {}
}
