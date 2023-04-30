//! A rich text component managing bevy a single [`Text`].
//!
//! As with the rest of `bevy_cuicui`, you can either build one by parsing
mod color;
mod integrate;
pub mod modifiers;
mod parse;
mod section;
mod trait_nonsense;

use std::any::{Any, TypeId};
use std::fmt;

use bevy::prelude::Font as BevyFont;
use bevy::utils::{hashbrown, PassHash};
use bevy::{prelude::*, utils::HashMap};

pub use integrate::{RichTextBundle, RichTextData, RichTextSetter, RichTextSetterItem};
pub use modifiers::{Color, Content, Dynamic, Font, RelSize};
pub use parse::Error as ParseError;
pub use section::Section;

pub type ModifyBox = Box<dyn Modify + Send + Sync + 'static>;
pub type Modifiers = HashMap<TypeId, ModifyBox>;
// here we want to own the `dyn Modify`, we might potentially be able to "cache"
// it and modify it in place with new values.
// TODO(arch): Maybe merge Bindings and TypeBindings into HashMap<(TypeId, Option<&str>), ModifyBox>
// TODO(clean): This relies on TypeId being a u64, which is BAAADDD
// TODO(perf): use some form of interning, or actually phf.
pub type Bindings = HashMap<&'static str, ModifyBox>;
pub type TypeBindings = hashbrown::HashMap<TypeId, ModifyBox, PassHash>;

/// A [`TextSection`] modifier.
///
/// A [`TextSection`] may have an arbitary number of `Modify`s, modifying
/// the styling and content of a given section.
pub trait Modify {
    // TODO(err): error handling (ie missing dynamic modifer binding)
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()>;

    // TODO(perf): used at the end of `richtext::parser::helpers::elements_and_content`
    // to propagate modifiers to nested text segments. Can't use `Modify: Clone`
    // since we need to work on trait objects and clone is not object-safe.
    // The alternative of using bevy's reflect is painful, since this would require
    // `ReflectFromReflect` and access to the type registry where the modifiers would
    // be pre-registered.
    // See todo in [`section`] for potential implementations.
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
// NOTE: an alternative implementation avoiding `Modify::clone_dyn` but requiring
// bevy reflect's TypeRegistry:
// use bevy::reflect::{ReflectFromReflect, TypeRegistryInternal as TypeRegistry};
// impl dyn Modify {
//     fn clone_reflect(&self, registry: &TypeRegistry) -> Box<Self> {
//         let registration = registry.get_with_name(self.type_name()).unwrap();
//         let rfr = registration.data::<ReflectFromReflect>().unwrap();
//         rfr.from_reflect(self.as_reflect()).unwrap()
//     }
// }
impl Clone for ModifyBox {
    fn clone(&self) -> Self {
        self.clone_dyn()
    }
}

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
    pub fonts: &'a dyn Fn(&str) -> Option<Handle<BevyFont>>,
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

#[derive(Debug)]
pub struct RichText {
    // TODO: this might be improved, for example by storing a binding-> section
    // list so as to avoid iterating over all sections when updating
    pub sections: Vec<Section>,
}

impl RichText {
    // /// Check if a type binding exists for given type
    // pub fn has_of<T: Any>(&self) -> bool {
    //     todo!()
    // }
    // /// Return the list of named bindings for a given type.
    // pub fn bindings_of<T: Any>(&self) -> impl Iterator<Item = &str> {
    //     self.sections.iter().flat_map(|s| &s.modifiers)
    //         .map(|m|
    // }

    /// Default cuicui rich text parser. Using a syntax inspired by rust's `format!` macro.
    ///
    /// See [rust doc](https://doc.rust-lang.org/stable/std/fmt/index.html).
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Ok(parse::rich_text(input)?)
    }
    // TODO(text): consider RichText independent from entity, might control several
    pub fn update(&self, to_update: &mut Text, ctx: &Context) {
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
