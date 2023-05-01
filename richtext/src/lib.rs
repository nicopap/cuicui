//! A rich text component managing bevy a single [`Text`].

mod gold_hash;
pub mod modifiers;
pub mod modify;
mod parse;
mod plugin;
pub mod track;

use std::any::TypeId;

use bevy::prelude::*;

use modifiers::Dynamic;

pub use modify::{IntoModify, Modifiers, Modify, ModifyBox};
pub use parse::Error as ParseError;
pub use track::{AppResourceTrackerExt, ResTrackers, Tracked};

// TODO(perf): See design_doc/richtext/better_section_impl.md
// TODO(perf): should have change tracking (might require internal mutability)
// to be precise and extremely limited about what we update.
// TODO(clean): should separate Content from other modifiers, since there is always
// exactly one per section (I kept it as Modifier because I can re-use Dynamic)
#[derive(PartialEq, Debug, Default)]
pub struct Section {
    modifiers: Modifiers,
}

#[derive(Debug)]
pub struct RichText {
    // TODO(perf): this might be improved, for example by storing a binding-> section
    // list so as to avoid iterating over all sections when updating
    pub sections: Vec<Section>,
}

impl RichText {
    fn any_section(&self, id: TypeId, f: impl Fn(Option<&Dynamic>) -> bool) -> bool {
        self.sections
            .iter()
            .flat_map(|mods| mods.modifiers.get(&id))
            .any(|modifier| f(modifier.as_any().downcast_ref()))
    }
    /// Check if a type binding exists for given type
    pub fn has_type_binding(&self, id: TypeId) -> bool {
        // TODO(perf): probably can do better.
        self.any_section(id, |modifier| matches!(modifier, Some(&Dynamic::ByType(_))))
    }

    /// Check if a named binding exists, and has the provided type.
    pub fn has_binding(&self, binding: &str, id: TypeId) -> bool {
        // TODO(perf): probably can do better.
        self.any_section(id, |modifier| {
            let Some(Dynamic::ByName(name)) = modifier else { return false; };
            &**name == binding
        })
    }

    /// Default cuicui rich text parser. Using a syntax inspired by rust's `format!` macro.
    ///
    /// See [rust doc](https://doc.rust-lang.org/stable/std/fmt/index.html).
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Ok(parse::rich_text(input)?)
    }
    // TODO(text): consider RichText independent from entity, might control several
    pub fn update(&self, to_update: &mut Text, ctx: &modify::Context) {
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
