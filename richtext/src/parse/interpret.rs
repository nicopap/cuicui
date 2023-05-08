use std::{
    any::{Any, TypeId},
    borrow::Cow,
};

use anyhow::Error as AnyError;
use bevy::utils::HashMap;
use thiserror::Error;

use super::structs::{Binding, Dyn, Format, Modifier, Section as ParseSection};
use crate::{
    modifiers::Content, modifiers::Dynamic, track::make_tracker, track::Tracker, Modifiers, Modify,
    ModifyBox, Section,
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

#[derive(Default)]
pub(crate) struct Context {
    // TODO(perf) see design_docs/richtext/hlist_interpreting.md
    pub(crate) modify_builders: HashMap<&'static str, (TypeId, MakeModifyBox)>,
}
impl Context {
    pub(crate) fn insert<T: Any + Modify>(&mut self) {
        self.modify_builders
            .insert(T::name(), (TypeId::of::<T>(), |i| T::parse(&i)));
    }
    pub(crate) fn richtext_defaults() -> Self {
        use crate::modifiers;

        let mut ret = Self::default();
        ret.insert::<modifiers::Content>();
        ret.insert::<modifiers::RelSize>();
        ret.insert::<modifiers::Font>();
        ret.insert::<modifiers::Color>();
        ret
    }
}
/// Turn a [`ParseSection`], a simple textual representation, into a [`Section`],
/// a collection of trait objects used for formatting.
pub(super) fn section(
    input: ParseSection,
    ctx: &Context,
    trackers: &mut Vec<Tracker>,
) -> AnyResult<Section> {
    let dynbox = |d: Dynamic| -> AnyResult<ModifyBox> { Ok(Box::new(d)) };
    let mut parse_modify_value = |type_id, value, parse: MakeModifyBox| match value {
        Dyn::Dynamic(Binding::Type) => dynbox(Dynamic::ByType(type_id)),
        Dyn::Dynamic(Binding::Name(name)) => dynbox(Dynamic::ByName(name.to_string())),
        Dyn::Dynamic(Binding::Format { path, format }) => {
            let show = match format {
                Format::UserDefined(_) => todo!("TODO(feat): user-specified formatters"),
                Format::Fmt(format) => Box::new(format),
            };
            // TODO(err): unwrap
            let tracker = make_tracker(path.to_string(), path, show).unwrap();
            trackers.push(tracker);
            dynbox(Dynamic::ByName(path.to_string()))
        }
        Dyn::Static(value) => {
            let mut value: Cow<'static, str> = value.to_owned().into();
            escape_backslashes(&mut value);
            parse(value)
        }
    };
    let parse_modify = |Modifier { name, value }| {
        let err = || Error::UnknownModifier(name.to_string());
        let (type_id, parse) = ctx.modify_builders.get(name).ok_or_else(err)?;
        let modify = parse_modify_value(*type_id, value, *parse)?;
        Ok((*type_id, modify))
    };
    let mut modifiers = input
        .modifiers
        .into_iter()
        .map(parse_modify)
        .collect::<AnyResult<Modifiers>>()?;

    // TODO(feat): combine Content & Format.

    let content_id = TypeId::of::<Content>();
    let content_value = parse_modify_value(content_id, input.content, |i| Ok(Box::new(Content(i))));
    modifiers.insert(content_id, content_value?);

    Ok(Section { modifiers })
}
