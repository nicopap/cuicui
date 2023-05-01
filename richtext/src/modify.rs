use std::{any::Any, any::TypeId, fmt};

use bevy::{
    prelude::{Font, Handle, TextSection, TextStyle},
    utils::HashMap,
};

use crate::gold_hash::GoldMap;

/// A Boxed [`Modify`] trait object, with all necessary bounds to make it work
/// with bevy's [`Resource`] and [`Component`] types.
pub type ModifyBox = Box<dyn Modify + Send + Sync + 'static>;

pub type Modifiers = GoldMap<TypeId, ModifyBox>;

/// Turn any type into a [modifier](ModifyBox).
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
/// A [`TextSection`] may have an arbitary number of `Modify`s, modifying
/// the styling and content of a given section.
pub trait Modify {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()>;

    // TODO(perf): See design_doc/richtext/better_section_impl.md.
    /// Clone the value as a trait object.
    ///
    /// The following implementation should work:
    /// ```ignore
    /// fn clone_dyn(&self) -> super::ModifyBox {
    ///     Box::new(self.clone())
    /// }
    /// ```
    fn clone_dyn(&self) -> ModifyBox;
    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn Modify) -> bool;
    fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
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
