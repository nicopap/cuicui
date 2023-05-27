use core::fmt;

use bevy::reflect::Reflect;

use crate::{modifiers::Modifier, parse, show::RuntimeFormat};

// TODO(perf): If bindings::World had a `entry -> Entry` we could use it to
// optionally update in-place the TextModifier. This would help avoiding
// allocations.
/// Turn a [`&dyn Reflect`] into a [`TextModifier`].
pub enum Write {
    /// Print the `Reflect` as a [`TextModifier::content`] displayed with the
    /// given format specification.
    Format(RuntimeFormat),

    /// An arbitrary function to run on the `Reflect`.
    Arbitrary(fn(&dyn Reflect) -> Modifier),

    /// Print the `Reflect` as a [`TextModifier::content`] displayed with
    /// [`Reflect::debug`].
    Debug,
}
impl Write {
    pub fn modify(&self, value: &dyn Reflect) -> Modifier {
        let content = |s: String| Modifier::content(s.into());
        match self {
            Write::Format(fmt) => content(fmt.display(value).to_string()),
            Write::Arbitrary(run) => run(value),
            Write::Debug => content(DisplayReflect(value).to_string()),
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
