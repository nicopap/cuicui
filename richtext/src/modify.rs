//! Traits and structs related to the [`Modify`] trait.

use std::{any::Any, fmt};

use bevy::prelude::{Font, Handle, TextSection};
use enumset::EnumSetType;

pub use anyhow::Error as AnyError;
pub use enumset::EnumSet;

/// A Boxed [`Modify`] trait object, with all necessary bounds to make it work
/// with bevy's [`Resource`] and [`Component`] types.
///
/// [`Resource`]: bevy::prelude::Resource
/// [`Component`]: bevy::prelude::Component
pub type ModifyBox = Box<dyn Modify + Send + Sync + 'static>;

pub type GetFont<'a> = &'a dyn Fn(&str) -> Option<Handle<Font>>;

/// Turn a type into a boxed [`Modify`] trait object.
pub trait IntoModify {
    fn into_modify(self) -> ModifyBox;
}
impl<T: Modify + Send + Sync + 'static> IntoModify for T {
    fn into_modify(self) -> ModifyBox {
        Box::new(self)
    }
}
impl IntoModify for ModifyBox {
    fn into_modify(self) -> ModifyBox {
        self
    }
}

/// A [`TextSection`] modifier.
///
/// A rich text [`Section`] may have an arbitary number of `Modify`s, modifying
/// the styling and content of a given section.
///
/// # Implementing `Modify`
///
/// You can create your own modifiers, the `as_any`, `eq_dyn` and
/// `debug_dyn` cannot be implemented at the trait level due to rust's trait object
/// rules, but they should all look similar.
///
/// The `apply` method is what should be interesting for you.
///
/// ```rust
/// use std::{any::Any, fmt};
/// use bevy::prelude::*;
/// use cuicui_richtext::modify::{Modify, GetFont, ModifyBox, AnyError, Change, EnumSet};
///
/// #[derive(Debug, PartialEq, Clone, Copy)]
/// struct SetExactFontSize(f32);
///
/// impl Modify for SetExactFontSize {
///
///     /// Set the size of the text.
///     fn apply(&self, _: GetFont, text: &mut TextSection) -> Result<(), AnyError> {
///         text.style.font_size = self.0;
///         Ok(())
///     }
///     /// Declare when to update the text section.
///     fn depends(&self) -> EnumSet<Change> {
///         EnumSet::EMPTY // We depend on nothing.
///     }
///     /// Declare what `SetExactFontSize` changes.
///     fn changes(&self) -> EnumSet<Change> {
///         Change::FontSize.into()
///     }
///     fn as_any(&self) -> &dyn Any { self }
///     fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
///     fn eq_dyn(&self, other: &dyn Modify) -> bool {
///         let any = other.as_any();
///         let Some(right) = any.downcast_ref::<Self>() else { return false; };
///         self == right
///     }
/// }
/// ```
///
/// [`Section`]: crate::Section
pub trait Modify: Any {
    /// Apply this modifier to the `text`.
    ///
    /// [`RichText`]: crate::RichText
    fn apply(&self, fonts: GetFont, text: &mut TextSection) -> Result<(), AnyError>;

    /// On what data does this modifier depends?
    fn depends(&self) -> EnumSet<Change>;

    /// What data does this `Modify` changes?
    fn changes(&self) -> EnumSet<Change>;

    // TODO(feat): This should later be removed as `Dynamic` will not be a `Modify` anymore
    /// Which binding does this `Modify` depends on?
    fn binding(&self) -> Option<BindingId> {
        None
    }

    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn Modify) -> bool;
    fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}
pub trait Parse: Modify {
    /// The name to use when parsing metadata in the format string.
    ///
    /// **This must be formatted as an identifier** (ie: `[:alpha:_][:alphanum:_]*`).
    /// Otherwise, this won't be parsed correctly.
    ///
    /// By default, this is the name of your type.
    ///
    /// The default implementation should cause a compile time error if `Self`
    /// has generic parameters. In which case, you should provide your own
    /// implementation.
    ///
    /// You may overwrite this method regardless, as long as the return value
    /// is an identifier.
    const NAME: &'static str;

    /// Parse from the string representation of the `metadata` value section
    /// of the format string.
    ///
    /// When parsing a format string, we call `Modify::parse` of registered
    /// `Modify` types which name we encounter in the `key` metadata position.
    fn parse(input: &str) -> Result<ModifyBox, AnyError>;
}
impl PartialEq for dyn Modify {
    fn eq(&self, other: &Self) -> bool {
        self.eq_dyn(other)
    }
}
impl PartialEq for ModifyBox {
    fn eq(&self, other: &Self) -> bool {
        self.eq_dyn(&**other)
    }
}
impl fmt::Debug for dyn Modify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_dyn(f)
    }
}
impl fmt::Debug for ModifyBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_dyn(f)
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BindingId(pub(crate) u32);
impl fmt::Debug for BindingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<B{}>", self.0)
    }
}

/// On what value in [`Context`] does this [`Modify`] depends on?
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependsOn {
    Binding(BindingId),
    Parent(Change),
}
#[derive(EnumSetType, Debug, PartialOrd, Ord)]
pub enum Change {
    FontSize,
    Font,
    Color,
}
