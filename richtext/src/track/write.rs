use std::fmt;

use bevy::reflect::Reflect;
use fab::binding;

use crate::{modifiers::Modifier, parse, show::RuntimeFormat};

/// Turn a [`&dyn Reflect`] into a [`TextModifier`].
pub enum Write {
    /// Print the `Reflect` as a [`TextModifier::content`] displayed with the
    /// given format specification.
    Format(RuntimeFormat),

    /// An arbitrary function to run on the `Reflect`.
    Arbitrary(fn(&dyn Reflect, binding::Entry<Modifier>)),

    /// Print the `Reflect` as a [`TextModifier::content`] displayed with
    /// [`Reflect::debug`].
    Debug,
}
impl Write {
    pub fn modify(&self, value: &dyn Reflect, entry: binding::Entry<Modifier>) {
        fn set_content(entry: binding::Entry<Modifier>, s: impl fmt::Display) {
            entry
                .modify(|m| m.overwrite_content(&s))
                .or_insert(Modifier::content(s.to_string().into()));
        }
        match self {
            Write::Format(fmt) => set_content(entry, fmt.display(value)),
            Write::Arbitrary(run) => run(value, entry),
            Write::Debug => set_content(entry, DisplayReflect(value)),
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
struct DisplayReflect<'a>(&'a dyn Reflect);
impl fmt::Display for DisplayReflect<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.debug(f)
    }
}
