use std::sync::{Arc, TryLockError};
use std::{fmt, sync::Mutex};

use bevy::prelude::World;
use bevy::{reflect::Reflect, utils::HashMap};
use fab::binding::{self, Id};
use fab_parse::{hook::Format, RuntimeFormat};
use thiserror::Error;

use crate::fmt_system::IntoFmtSystem;
use crate::{fmt_system::FmtSystem, BevyModify};

#[derive(Debug, Error)]
pub enum UserWriteError {
    #[error("Can't lock a fmt system, this is a cuicui bug, please open an issue")]
    Locked,
    #[error("Somehow a thread locking on 'formatter' panicked, this is a cuicui bug, please open an issue")]
    Poisoned,
}
impl<T> From<TryLockError<T>> for UserWriteError {
    fn from(value: TryLockError<T>) -> Self {
        match value {
            TryLockError::Poisoned(_) => UserWriteError::Poisoned,
            TryLockError::WouldBlock => UserWriteError::Locked,
        }
    }
}

/// A writer defined by the user, it allows converting arbitrary values into `M` modifiers.
pub enum UserFmt<M> {
    // TODO(feat): Allow failure
    System(Arc<Mutex<dyn FmtSystem<M>>>),
    Function(Arc<dyn Fn(&dyn Reflect, binding::Entry<'_, M>) + Send + Sync>),
}
impl<M> UserFmt<M> {
    pub fn from_system<T: FmtSystem<M>>(
        system: impl IntoFmtSystem<M, T>,
        world: &mut World,
    ) -> Self {
        UserFmt::System(Arc::new(Mutex::new(system.into_fmt_system(world))))
    }
    pub fn from_fn(
        dyn_fn: impl Fn(&dyn Reflect, binding::Entry<M>) + Send + Sync + 'static,
    ) -> Self {
        UserFmt::Function(Arc::new(dyn_fn))
    }
    fn arc_clone(&self) -> Self {
        match self {
            UserFmt::System(sys) => UserFmt::System(Arc::clone(sys)),
            UserFmt::Function(dyn_fn) => UserFmt::Function(Arc::clone(dyn_fn)),
        }
    }
    fn run_system(
        &self,
        value: &dyn Reflect,
        entry: binding::Entry<M>,
        world: &World,
    ) -> Result<(), UserWriteError>
    where
        M: 'static,
    {
        match self {
            UserFmt::System(locked_sys) => locked_sys.try_lock()?.run(value, entry, world),
            UserFmt::Function(dyn_fn) => dyn_fn(value, entry),
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Formatter not found: {0:?}")]
    NotFormatter(Id),
}

// TODO(perf): use a `IndexMap` when I get around to implement it.
pub(crate) struct UserFmts<M>(HashMap<Id, UserFmt<M>>);

impl<M> UserFmts<M> {
    fn get(&self, binding: &Id) -> Option<UserFmt<M>> {
        self.0.get(binding).map(UserFmt::arc_clone)
    }
    pub fn new() -> Self {
        UserFmts(HashMap::new())
    }
    pub fn insert(&mut self, binding: Id, value: UserFmt<M>) -> Option<UserFmt<M>> {
        self.0.insert(binding, value)
    }
}

/// Turn a [`&dyn Reflect`] into a [`BevyModify`].
pub enum Write<M> {
    /// Print the [`Reflect`] as a [`BevyModify::set_content`] displayed with the
    /// given format specification.
    Format(RuntimeFormat),

    /// An arbitrary function to run on the [`Reflect`].
    Arbitrary(UserFmt<M>),

    /// Print the [`Reflect`] as a [`BevyModify::set_content`] displayed with
    /// [`Reflect::debug`].
    Debug,
}
impl<M: BevyModify> Write<M> {
    pub fn modify(&self, world: &World, value: &dyn Reflect, entry: binding::Entry<M>) {
        match self {
            Write::Format(fmt) => set_content(entry, &DisplayReflect(value, Some(fmt))),
            Write::Arbitrary(run) => run.run_system(value, entry, world).unwrap(),
            Write::Debug => set_content(entry, &DisplayReflect(value, None)),
        }
    }

    pub(crate) fn from_parsed(
        format: Option<Format>,
        provided: &UserFmts<M>,
    ) -> Result<Self, Error> {
        match format {
            None => Ok(Write::Debug),
            Some(Format::Fmt(format)) => Ok(Write::Format(format)),
            Some(Format::UserDefined(binding)) => provided
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
