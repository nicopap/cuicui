use std::fmt;

use bevy::reflect::Reflect;
use fab::binding;
use fab_parse::{tree as parse, RuntimeFormat};

use crate::BevyModify;

/// Turn a [`&dyn Reflect`] into a [`TextModifier`].
pub enum Write<M> {
    /// Print the `Reflect` as a [`TextModifier::content`] displayed with the
    /// given format specification.
    Format(RuntimeFormat),

    /// An arbitrary function to run on the `Reflect`.
    Arbitrary(fn(&dyn Reflect, binding::Entry<M>)),

    /// Print the `Reflect` as a [`TextM::content`] displayed with
    /// [`Reflect::debug`].
    Debug,
}
impl<M: BevyModify> Write<M> {
    pub fn modify(&self, value: &dyn Reflect, entry: binding::Entry<M>) {
        match self {
            // TODO(feat): Proper runtime formatter
            Write::Format(fmt) => set_content(entry, &DisplayReflect(value, Some(fmt))),
            Write::Arbitrary(run) => run(value, entry),
            Write::Debug => set_content(entry, &DisplayReflect(value, None)),
        }
    }

    pub(crate) fn from_parsed(format: Option<parse::Format>) -> Self {
        match format {
            Some(parse::Format::Fmt(format)) => Write::Format(format),
            Some(parse::Format::UserDefined(_)) => todo!(),
            None => Write::Debug,
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
