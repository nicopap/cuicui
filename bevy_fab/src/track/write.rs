use std::fmt;
use std::sync::Arc;

use bevy::{reflect::Reflect, utils::HashMap};
use fab::binding;
use fab_parse::{hook as parse, RuntimeFormat};
use thiserror::Error;

use crate::BevyModify;

/// A writer defined by the user, it allows converting arbitrary values into `M` modifiers.
pub type UserWrite<M> = Arc<dyn Fn(&dyn Reflect, binding::Entry<M>) + Send + Sync>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Formatter not found: {0:?}")]
    NotFormatter(binding::Id),
}

// TODO(perf): use a `IndexMap` when I get around to implement it.
pub(crate) struct UserWrites<M>(HashMap<binding::Id, UserWrite<M>>);

impl<M> UserWrites<M> {
    fn get(&self, binding: &binding::Id) -> Option<UserWrite<M>> {
        self.0.get(binding).map(Arc::clone)
    }
    pub fn new() -> Self {
        UserWrites(HashMap::new())
    }
    pub fn insert(&mut self, binding: binding::Id, value: UserWrite<M>) -> Option<UserWrite<M>> {
        self.0.insert(binding, value)
    }
}

/// Turn a [`&dyn Reflect`] into a [`BevyModify`].
pub enum Write<M> {
    /// Print the [`Reflect`] as a [`BevyModify::set_content`] displayed with the
    /// given format specification.
    Format(RuntimeFormat),

    /// An arbitrary function to run on the [`Reflect`].
    Arbitrary(UserWrite<M>),

    /// Print the [`Reflect`] as a [`BevyModify::set_content`] displayed with
    /// [`Reflect::debug`].
    Debug,
}
impl<M: BevyModify> Write<M> {
    pub fn modify(&self, value: &dyn Reflect, entry: binding::Entry<M>) {
        match self {
            Write::Format(fmt) => set_content(entry, &DisplayReflect(value, Some(fmt))),
            Write::Arbitrary(run) => run(value, entry),
            Write::Debug => set_content(entry, &DisplayReflect(value, None)),
        }
    }

    pub(crate) fn from_parsed(
        format: Option<parse::Format>,
        provided: &UserWrites<M>,
    ) -> Result<Self, Error> {
        match format {
            None => Ok(Write::Debug),
            Some(parse::Format::Fmt(format)) => Ok(Write::Format(format)),
            Some(parse::Format::UserDefined(binding)) => provided
                .get(&binding)
                .map(Write::Arbitrary)
                .ok_or(Error::NotFormatter(binding)),
        }
    }
}
fn set_content<M: BevyModify>(entry: binding::Entry<M>, s: &impl fmt::Display) {
    entry
        .modify(|m| m.set_content(format_args!("{s}")))
        .or_insert_with(|| M::init_content(format_args!("{s}")));
}
struct DisplayReflect<'a>(&'a dyn Reflect, Option<&'a RuntimeFormat>);
impl fmt::Display for DisplayReflect<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(fmt) = self.1 {
            if let Ok(()) = fmt.display(self.0.as_any()).fmt(f) {
                return Ok(());
            }
        }
        self.0.debug(f)
    }
}
