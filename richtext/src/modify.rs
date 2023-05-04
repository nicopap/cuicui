//! Traits and structs related to the [`Modify`] trait.

use std::{any::type_name, any::Any, any::TypeId, fmt};

use bevy::{
    prelude::{Font, Handle, TextSection, TextStyle},
    reflect::TypeRegistryInternal as TypeRegistry,
    utils::HashMap,
};
use thiserror::Error;

use crate::{gold_hash::GoldMap, short_name::short_name};

pub use anyhow::Error as AnyError;

/// A Boxed [`Modify`] trait object, with all necessary bounds to make it work
/// with bevy's [`Resource`] and [`Component`] types.
///
/// [`Resource`]: bevy::prelude::Resource
/// [`Component`]: bevy::prelude::Component
pub type ModifyBox = Box<dyn Modify + Send + Sync + 'static>;
pub type Modifiers = GoldMap<TypeId, ModifyBox>;

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

#[derive(Error, Debug)]
#[error("The modify type for {0} is not implemented")]
struct ParserUnimplemented(&'static str);

/// A [`TextSection`] modifier.
///
/// A rich text [`Section`] may have an arbitary number of `Modify`s, modifying
/// the styling and content of a given section.
///
/// # Implementing `Modify`
///
/// You can create your own modifiers, the `clone_dyn`, `as_any`, `eq_dyn` and
/// `debug_dyn` cannot be implemented at the trait level due to rust's trait object
/// rules, but they should all look similar.
///
/// The `apply` method is what should be interesting for you.
///
/// ```rust
/// use std::{any::Any, fmt};
/// use bevy::prelude::*;
/// use cuicui_richtext::modify::{Modify, Context, ModifyBox, AnyError};
///
/// #[derive(Debug, PartialEq, Clone, Copy)]
/// struct SetExactFontSize(f32);
///
/// impl Modify for SetExactFontSize {
///
///     /// Set the size of the text.
///     fn apply(&self, ctx: &Context, text: &mut TextSection) -> Result<(), AnyError> {
///         text.style.font_size = self.0;
///         Ok(())
///     }
///     fn clone_dyn(&self) -> ModifyBox { Box::new(self.clone()) }
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
    /// Apply this modifier to the `text`, given a [`Context`].
    ///
    /// Note that the order of application of modifiers in [`RichText`] is
    /// **unspecified**, so you need to make sure your [`Modify`] is
    /// order-independent.
    ///
    /// [`RichText`]: crate::RichText
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Result<(), AnyError>;

    // TODO(perf): See design_doc/richtext/better_section_impl.md.
    fn clone_dyn(&self) -> ModifyBox;
    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn Modify) -> bool;
    fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    // TODO(clean): Drop the Option: if not meant to be parsed, then user
    // shouldn't add to `RichText` parser builder.
    /// The name to use when parsing metadata in [`RichText::parse`].
    ///
    /// `None` when this isn't supposed to be parsed.
    ///
    /// **This must be formatted as an identifier** (ie: `[:alpha:_][:alphanum:_]*`).
    /// Otherwise, the [`RichText::parse`]-ing will not pick up your modifier.
    ///
    /// By default, this is the name of your type.
    ///
    /// The default implementation should cause a compile time error if `Self`
    /// has generic parameters. In which case, you should provide your own
    /// implementation.
    ///
    /// You may overwrite this method regardless, as long as the return value
    /// is an identifier.
    #[inline]
    fn name() -> Option<&'static str>
    where
        Self: Sized,
    {
        Some(short_name(type_name::<Self>()))
    }
    /// Parse from the string representation of the `metadata` value section
    /// of the format string.
    ///
    /// When parsing a format string, we call `Modify::parse` of registered
    /// `Modify` types which name we encounter in the `key` metadata position.
    ///
    /// By default, this returns a `ParserUnimplemented` error. Make sure to
    /// impelment it yourself if you intend on parsing metadata.
    fn parse(_input: &str) -> Result<ModifyBox, AnyError>
    where
        Self: Sized,
    {
        match Self::name() {
            Some(name) => Err(ParserUnimplemented(name).into()),
            None => unreachable!(
                "Parsers without names cannot be called, \
                in fact if the rust compiler is intelligent enough, this string \
                shouldn't be in your final binary."
            ),
        }
    }
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
impl Clone for ModifyBox {
    fn clone(&self) -> Self {
        self.clone_dyn()
    }
}

// TODO(arch): Maybe merge Bindings and TypeBindings into HashMap<(TypeId, Option<&str>), ModifyBox>
// TODO(perf): use interning or phf. see http://0x80.pl/notesen/2023-04-30-lookup-in-strings.html
// TODO(arch): This &'static str can be a limitation, thought not too bad, since
// bindings mostly happen at startup and we can deal with some box leaking.
pub type Bindings = HashMap<&'static str, ModifyBox>;
pub type TypeBindings = GoldMap<TypeId, ModifyBox>;

// TODO(doc): more details, explain bidings.
/// The context used in [`Modify`].
#[derive(Clone, Copy)]
pub struct Context<'a, 'b> {
    pub registry: Option<&'b TypeRegistry>,
    pub bindings: Option<&'b Bindings>,
    pub world_bindings: Option<&'b Bindings>,
    pub type_bindings: Option<&'b TypeBindings>,
    pub parent_style: &'b TextStyle,
    // NOTE: we use a `&'a dyn` here instead of a type parameter because we intend
    // for `Context` to be a parameter for a trait object method. If `Context` had
    // a non-lifetime type parameter, it would require that method to have a type
    // parameter itself, but this would make it non-dispatchable: ie not available
    // on trait object.
    pub fonts: &'a dyn Fn(&str) -> Option<Handle<Font>>,
}
impl<'a, 'b> Context<'a, 'b> {
    pub fn from_style(parent_style: &'b TextStyle) -> Self {
        Context {
            registry: None,
            bindings: None,
            world_bindings: None,
            type_bindings: None,
            parent_style,
            fonts: &|_| None,
        }
    }
}
