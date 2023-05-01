//! Default implementations of the [`TextMod`] trait for cuicui.
use std::{any::Any, any::TypeId, borrow::Cow, fmt};

use bevy::prelude::{FromReflect, Reflect, TextSection};
use bevy::reflect::ReflectFromReflect;

use crate::{modify::Context, IntoModify, Modify, ModifyBox};

macro_rules! common_modify_methods {
    () => {
        fn clone_dyn(&self) -> super::ModifyBox {
            Box::new(self.clone())
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn eq_dyn(&self, other: &dyn Modify) -> bool {
            let any = other.as_any();
            let Some(right) = any.downcast_ref::<Self>() else { return false; };
            self == right
        }
        fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use std::fmt::Debug;
            self.fmt(f)
        }
    };
}

/// A font name.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Font(pub String);
impl Modify for Font {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new font: {:?}", self.0);
        text.style.font = (ctx.fonts)(&self.0)?;
        Some(())
    }
    common_modify_methods! {}
}

/// Size relative to global text size.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct RelSize(pub f32);
impl Modify for RelSize {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new font size: {:?}", self.0);
        text.style.font_size = ctx.parent_style.font_size * self.0;
        Some(())
    }
    common_modify_methods! {}
}

/// Color.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Color(pub bevy::prelude::Color);
impl Modify for Color {
    fn apply(&self, _ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new color: {:?}", self.0);
        text.style.color = self.0;
        Some(())
    }
    common_modify_methods! {}
}
impl IntoModify for bevy::prelude::Color {
    fn into_modify(self) -> ModifyBox {
        Box::new(Color(self))
    }
}

/// A section text, may either be preset or extracted.
#[derive(Reflect, PartialEq, Debug, Clone, FromReflect)]
#[reflect(FromReflect)]
pub struct Content(pub Cow<'static, str>);
impl Modify for Content {
    fn apply(&self, _ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Apply new content: {:?}", self.0);
        text.value.clear();
        text.value.push_str(&self.0);
        Some(())
    }
    common_modify_methods! {}
}
impl<T: fmt::Display> From<T> for Content {
    fn from(value: T) -> Self {
        Content(value.to_string().into())
    }
}

// TODO(text): most likely could use interning for Dynamic section text name.
// this would involve replacing Strings with an enum String|Interned, or private
// background components, otherwise API seems impossible.
/// An [`Modify`] that takes it value from [`Context::bindings`].
#[derive(PartialEq, Debug, Clone)]
pub enum Dynamic {
    ByName(String),
    // TODO(clean): remove `TypeId` here, since it is necessarily associated
    // with a `TypeId` when inserted into the `Modifiers` map.
    // Probably need to add the `TypeId` to `Context`.
    ByType(TypeId),
}
impl Dynamic {
    pub fn new(name: String) -> Self {
        Dynamic::ByName(name)
    }
}
impl Modify for Dynamic {
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Option<()> {
        // println!("Get value from binding: {:?}", self.name);
        let modifier = match self {
            Dynamic::ByName(name) => ctx.bindings?.get(&**name),
            Dynamic::ByType(type_id) => ctx.type_bindings?.get(type_id),
        };
        modifier?.apply(ctx, text)
    }
    common_modify_methods! {}
}
impl Modify for () {
    fn apply(&self, _: &Context, _: &mut TextSection) -> Option<()> {
        Some(())
    }
    common_modify_methods! {}
}
