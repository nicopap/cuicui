use std::{any::Any, borrow::Cow};

use anyhow::Error as AnyError;
use bevy::utils::HashMap;
use fab::binding;
use thiserror::Error;

use crate::modifiers::{self, ModifyBox};
use crate::richtext::TextPrefab;

#[derive(Error, Debug)]
pub(super) enum Error {
    #[error("Tried to use an unregistered modifier: {0}")]
    UnknownModifier(String),
}

pub(crate) type MakeModifyBox = fn(Cow<'static, str>) -> Result<ModifyBox, AnyError>;

pub(crate) struct Context<'a> {
    // TODO(perf) see design_docs/richtext/hlist_interpreting.md + consider interning
    pub(super) modify_builders: HashMap<&'static str, MakeModifyBox>,
    pub(crate) bindings: &'a mut binding::World<TextPrefab>,
}
impl<'a> Context<'a> {
    fn insert<T: Any + modifiers::Parse>(&mut self) {
        self.modify_builders.insert(T::NAME, |i| T::parse(&i));
    }
    pub(crate) fn new(bindings: &'a mut binding::World<TextPrefab>) -> Self {
        Self { modify_builders: HashMap::default(), bindings }
    }
    pub(crate) fn with_defaults(mut self) -> Self {
        self.insert::<modifiers::Content>();
        self.insert::<modifiers::RelSize>();
        self.insert::<modifiers::Font>();
        self.insert::<modifiers::Color>();

        self
    }
}
