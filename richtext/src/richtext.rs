use std::{any::TypeId, fmt};

use bevy::{
    prelude::{debug, warn, Text, TextSection},
    reflect::{Reflect, Typed},
    utils::HashMap,
};

use crate::{
    modifiers::Dynamic, modify, parse, parse::interpret, show, show::ShowBox, AnyError, Modifiers,
    Tracker,
};

// TODO(perf): See design_doc/richtext/better_section_impl.md
// TODO(perf): should have change tracking (might require internal mutability)
// to be precise and extremely limited about what we update.
#[derive(PartialEq, Debug, Default)]
pub struct Section {
    pub(crate) modifiers: Modifiers,
}

pub struct RichTextBuilder {
    format_string: String,
    context: interpret::Context,
    // TODO(perf): This sucks, the `FetchBox`, which we are using this for, is
    // calling itself the `ShowBox` impl. Instead of storing formatters, we should
    // directly construct the `FetchBox` when it is added
    // TODO(feat): This is actually unused.
    formatters: HashMap<&'static str, ShowBox>,
}
impl RichTextBuilder {
    pub fn new_no_defaults(format_string: impl Into<String>) -> Self {
        RichTextBuilder {
            format_string: format_string.into(),
            context: interpret::Context::default(),
            formatters: HashMap::default(),
        }
    }
    /// Default cuicui rich text parser, [see the syntax].
    ///
    /// [see the syntax]: https://github.com/nicopap/cuicui/blob/main/design_doc/richtext/informal_grammar.md
    pub fn new(format_string: impl Into<String>) -> Self {
        RichTextBuilder {
            format_string: format_string.into(),
            context: interpret::Context::richtext_defaults(),
            formatters: HashMap::default(),
        }
    }
    /// Add a [formatter](crate::show::Show).
    pub fn fmt<I, O, F>(mut self, name: &'static str, convert: F) -> Self
    where
        I: Reflect + Typed,
        O: fmt::Display + 'static, // TODO(bug): shouldn't need this + 'static
        F: Clone + Send + Sync + Fn(&I) -> O + 'static,
    {
        self.formatters
            .insert(name, show::Convert::<I, O, F>::new(convert));
        self
    }
    pub fn build(self) -> Result<(RichText, Vec<Tracker>), AnyError> {
        let Self { format_string, context, .. } = self;
        let mut trackers = Vec::new();
        let sections = parse::richtext(context, &format_string, &mut trackers)?;
        let partial = RichTextPartial { sections };

        debug!("Making RichText: {format_string:?}");
        partial.print_bindings();

        Ok((partial.consume(), trackers))
    }
}

struct RichTextPartial {
    sections: Vec<Section>,
}

impl RichTextPartial {
    fn consume(self) -> RichText {
        RichText { sections: self.sections }
    }
    fn print_bindings(&self) {
        debug!("Bindings are:");
        for name in self.dynamic_binding_names() {
            debug!("\t{name}");
        }
    }
    fn dynamic_binding_names(&self) -> impl Iterator<Item = &str> {
        self.sections.iter().flat_map(|section| {
            let values = || section.modifiers.values();
            if let Some(Dynamic::ByName(name)) = values().find_map(|m| m.as_any().downcast_ref()) {
                Some(name.as_str())
            } else {
                None
            }
        })
    }
}

#[derive(Debug)]
pub struct RichText {
    // TODO(perf): this might be improved, for example by storing a binding-> section
    // list so as to avoid iterating over all sections when updating
    sections: Vec<Section>,
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

    // TODO(feat): consider RichText independent from entity, might control several
    pub fn update(&self, to_update: &mut Text, ctx: &modify::Context) {
        to_update.sections.resize_with(self.sections.len(), || {
            TextSection::from_style(ctx.parent_style.clone())
        });

        let rich = self.sections.iter();
        let poor = to_update.sections.iter_mut();

        for (to_set, value) in poor.zip(rich) {
            for modifier in value.modifiers.values() {
                if let Err(err) = modifier.apply(ctx, to_set) {
                    warn!("error when applying modifiers: {err}");
                }
            }
        }
    }
}
