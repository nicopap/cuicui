//! Define the [`track!`] macro.

/// Add a component and keep track of its value in [`WorldBindings`],
/// this is a soft wrapper around [`TrackerBundle`] methods.
///
/// # Syntax
///
/// `track!` supports several call convention:
///
/// - `track!('X, <identifier>, <expression>)`
/// - `track!('X, <identifier>)`
/// - `track!(<identifier>, <expression>)`
/// - `track!(<identifier>)`
///
/// The first (optional) parameter is a `'label`. It can have one of
/// the following values:
///
/// - `'c` (default if omitted): See [`TrackerBundle::content`].
///   Track the component as a content binding.
///   This requires the component to implement [`fmt::Display`].
/// - `'d`: See [`TrackerBundle::debug`]. Track the component as a debug content binding.
///   This requires the component to implement [`fmt::Debug`].
///   If the cargo feature `no_tracked_debug` is enabled, it will just insert the component.
/// - `'m`: See [`TrackerBundle::modifier`].
///   Track the component as a modifier binding.
///   It must implement [`IntoModify`] and [`Clone`].
///
/// The second parameter is a rust identifier. It will be the name of the binding
/// in [`WorldBindings`] the component will have.
///
/// The third parameter is the component as it should be added to the entity.
/// If omitted, `track!` will use the second parameters value instead, assuming
/// that it is a variable containing the component as value.
///
/// ## Examples
///
/// ```
/// use bevy::prelude::*;
/// use cuicui_richtext::track;
///
/// # #[derive(Component, Default)] struct MaxValue(f32);
/// # #[derive(Component, Default)] struct MinValue(f32);
/// # #[derive(Component, Default, Reflect, Debug)]
/// # struct Slider(f32);
/// #
/// # #[derive(Bundle, Default)]
/// # struct NonsliderBundle {
/// #     max: MaxValue,
/// #     min: MinValue,
/// # }
/// fn setup(mut commands: Commands) {
///     let value = 11.0;
///     commands.spawn((
///         NonsliderBundle { max: MaxValue(34.0), ..default() },
///         track!('d, slider, Slider(value)),
///     ));
/// }
/// ```
///
/// Note that if the component you want to track is already part of a bundle,
/// you can work around it by inserting the tracked version afterward.
///
/// ```
/// use bevy::prelude::*;
/// use cuicui_richtext::track;
///
/// # #[derive(Component, Default)] struct MaxValue(f32);
/// # #[derive(Component, Default)] struct MinValue(f32);
/// # #[derive(Component, Default, Reflect, Debug)]
/// # struct Slider(f32);
/// # impl std::fmt::Display for Slider {
/// #    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:.3}", self.0) }
/// # }
/// #
/// # #[derive(Bundle, Default)]
/// # struct SliderBundle {
/// #     max: MaxValue,
/// #     min: MinValue,
/// #     slider: Slider,
/// # }
/// # fn setup(mut commands: Commands) {
/// let value = 11.0;
/// commands.spawn(
///     SliderBundle { max: MaxValue(100.0), slider: Slider(value), ..default() },
/// );
/// // Becomes:
/// commands.spawn(
///     SliderBundle { max: MaxValue(100.0), ..default() }
/// )
/// .insert(track!(slider, Slider(value)));
/// # }
/// ```
///
/// ## Implementation notes
///
/// The macro accepts a lifetime (`'d`) as first parameter because it's the only
/// thing that can be matched by name in macro expansion, outside of an identifier
/// and a generic token. But identifier is already used in this position, so
/// we chose a lifetime (I would have rather used a string literal).
#[macro_export]
macro_rules! track {
    (@ctor 'd) => { $crate::TrackerBundle::<_, $crate::Modifier>::debug };
    (@ctor 'm) => { $crate::TrackerBundle::<_, $crate::Modifier>::modifier };
    (@ctor 'c) => { $crate::TrackerBundle::<_, $crate::Modifier>::content };
    (@ctor ) => { $crate::TrackerBundle::<_, $crate::Modifier>::content };
    (@build($ctor:expr, $binding_name:expr, $component:expr)) => {
        ($ctor)($binding_name, $component)
    };
    ($( $flag:lifetime, )? $binding_name:ident , $component:expr)  => {{
        let ctor = track!(@ctor $($flag)?);
        track!(@build(ctor, stringify!($binding_name), $component))
    }};
    ($( $flag:lifetime, )? $variable_name:ident)  => {
        let ctor = track!(@ctor $($flag)?);
        track!(@build(ctor, stringify!($variable_name), $variable_name))
    };
}
