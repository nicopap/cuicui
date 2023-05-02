use std::{any::type_name, any::Any, any::TypeId, fmt};

use bevy::{
    prelude::{Font, Handle, TextSection, TextStyle},
    utils::HashMap,
};

use crate::{gold_hash::GoldMap, short_name::short_name};

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
/// use cuicui_richtext::modify::{Modify, Context, ModifyBox};
///
/// #[derive(Debug, PartialEq, Clone, Copy)]
/// struct SetExactFontSize(f32);
///
/// impl Modify for SetExactFontSize {
///
///     /// Set the size of the text.
///     fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
///         text.style.font_size = self.0;
///         Some(())
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
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()>;

    // TODO(perf): See design_doc/richtext/better_section_impl.md.
    fn clone_dyn(&self) -> ModifyBox;
    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn Modify) -> bool;
    fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    // TODO(feat): custom `Modify` for parsing
    /// **UNUSED** this doc string is purely prospective.
    ///
    /// The name to use when parsing metadata in [`RichText::parse`].
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
    fn name() -> &'static str
    where
        Self: Sized,
    {
        short_name(type_name::<Self>())
    }
    // fn parse(input: &str) -> Result<Self, ???> where Self: Sized;
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
    pub bindings: Option<&'b Bindings>,
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
            bindings: None,
            parent_style,
            fonts: &|_| None,
            type_bindings: None,
        }
    }
}
