use std::{any::Any, borrow::Cow};

use anyhow::Error as AnyError;
use bevy::utils::HashMap;
use thiserror::Error;

use super::structs::{Binding, Dyn, Format, Modifier as ParseModifier, Section as ParseSection};
use crate::{
    binding::WorldBindings, modifiers::Dynamic, modify, richtext::Modifier, track::make_tracker,
    track::Tracker, ModifyBox,
};

#[derive(Error, Debug)]
pub(super) enum Error {
    #[error("Tried to use an unregistered modifier: {0}")]
    UnknownModifier(String),
}

fn escape_backslashes(input: &mut Cow<str>) {
    if !input.contains('\\') {
        return;
    }
    let input = input.to_mut();
    let mut prev_normal = true;
    input.retain(|c| {
        let backslash = c == '\\';
        let remove = prev_normal && backslash;
        let normal = !remove;
        prev_normal = normal || !backslash;
        normal
    });
}
type AnyResult<T> = Result<T, AnyError>;
pub(crate) type MakeModifyBox = fn(Cow<'static, str>) -> Result<ModifyBox, AnyError>;

pub(crate) struct Context<'a> {
    // TODO(perf) see design_docs/richtext/hlist_interpreting.md + consider interning
    modify_builders: HashMap<&'static str, MakeModifyBox>,
    pub(crate) bindings: &'a mut WorldBindings,
}
impl<'a> Context<'a> {
    fn insert<T: Any + modify::Parse>(&mut self) {
        self.modify_builders.insert(T::NAME, |i| T::parse(&i));
    }
    pub(crate) fn new(bindings: &'a mut WorldBindings) -> Self {
        Self { modify_builders: HashMap::default(), bindings }
    }
    pub(crate) fn with_defaults(mut self) -> Self {
        use crate::modifiers;

        self.insert::<modifiers::Content>();
        self.insert::<modifiers::RelSize>();
        self.insert::<modifiers::Font>();
        self.insert::<modifiers::Color>();

        self
    }
}
/// Add modifiers from [`ParseSection`], a simple textual representation,
/// to `modifiers` the list of `Modifier`s for a entire `RichText`.
pub(super) fn section(
    section_index: usize,
    input: ParseSection,
    ctx: &mut Context,
    trackers: &mut Vec<Tracker>,
    modifiers: &mut Vec<Modifier>,
) -> AnyResult<()> {
    let mut dynbox = |name: &str| -> AnyResult<ModifyBox> {
        Ok(Box::new(Dynamic(ctx.bindings.get_or_add(name))))
    };
    let mut parse_modify_value = |value, parse: MakeModifyBox| match value {
        Dyn::Dynamic(Binding::Name(name)) => dynbox(name),
        Dyn::Dynamic(Binding::Format { path, format }) => {
            let show = match format {
                Format::UserDefined(_) => todo!("TODO(feat): user-specified formatters"),
                Format::Fmt(format) => Box::new(format),
            };
            // TODO(err): unwrap
            let tracker = make_tracker(path.to_string(), path, show).unwrap();
            trackers.push(tracker);
            dynbox(path)
        }
        Dyn::Static(value) => {
            let mut value: Cow<'static, str> = value.to_owned().into();
            escape_backslashes(&mut value);
            parse(value)
        }
    };
    let parse_modify = |ParseModifier { name, value, subsection_count }| {
        let err = || Error::UnknownModifier(name.to_string());
        let parse = ctx.modify_builders.get(name).ok_or_else(err)?;
        let try_u32 = u32::try_from;
        let modifier = Modifier {
            modify: parse_modify_value(value, *parse)?,
            range: try_u32(section_index)?..try_u32(section_index + subsection_count)?,
        };
        Ok(modifier)
    };
    modifiers.extend(
        input
            .modifiers
            .into_iter()
            // NOTE: `ParseModifier` are sorted in ascending order (most specific
            // to most general), but we want the reverse, most general to most specific,
            // so that, when we iterate through it, we can apply general, the specific etc.
            .rev()
            .map(parse_modify)
            // TODO(clean): Is it even possible to do this without the Vec?
            .collect::<AnyResult<Vec<_>>>()?,
    );
    Ok(())
}
