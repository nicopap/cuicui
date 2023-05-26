use std::fmt;

use winnow::error::{ContextError, ErrorKind, ParseError};

#[derive(Debug)]
pub struct Parse<I>(Vec<(I, InternalElem)>);

impl<I: fmt::Display> fmt::Display for Parse<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "I'd really like to make this error message more user-friendly.\n\
            But I need to first prioritize other features of cuicui_richtext.\n\
            Parse error:\n"
        )?;
        for (input, error) in &self.0 {
            match error {
                InternalElem::Context(s) => writeln!(f, "in section '{s}', at: {input}")?,
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(super) enum InternalElem {
    Context(&'static str),
}
impl<I> ParseError<I> for Parse<I> {
    fn from_error_kind(_: I, _: ErrorKind) -> Self {
        Parse(Vec::new())
    }
    fn append(self, _: I, _: ErrorKind) -> Self {
        self
    }
}
impl<I> ContextError<I> for Parse<I> {
    fn add_context(mut self, input: I, ctx: &'static str) -> Self {
        self.0.push((input, InternalElem::Context(ctx)));
        self
    }
}
