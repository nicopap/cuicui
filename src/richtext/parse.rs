//! Parse rich text according to spec
use std::{any::TypeId, num::ParseFloatError};

use thiserror::Error;

use super::{color, dynamic::Dyn, Color, Content, DynModifier, Font, Modifiers, RelSize, Section};

/// Whether text is within braces or not.
enum ContentType {
    Closed,
    Open,
}

#[derive(Copy, Clone, PartialEq)]
enum Modifier {
    Font,
    Color,
    RelSize,
    Content,
}
impl Modifier {
    fn type_id(&self) -> TypeId {
        match self {
            Modifier::Font => TypeId::of::<Font>(),
            Modifier::Color => TypeId::of::<Color>(),
            Modifier::RelSize => TypeId::of::<RelSize>(),
            Modifier::Content => TypeId::of::<Content>(),
        }
    }
}
#[derive(Copy, Clone, PartialEq)]
enum Flow {
    Dynamic,
    Static,
}
#[derive(Copy, Clone, PartialEq)]
struct Element<'a> {
    modifier: Modifier,
    flow: Flow,
    value: &'a str,
}
impl<'a> Element<'a> {
    fn parse_modifier(&self) -> Result<DynModifier> {
        Ok(match self.modifier {
            Modifier::Font => Box::new(Dyn::Set(Font(self.value.to_owned()))),
            Modifier::Color => Box::new(Dyn::Set(self.value.parse::<Color>()?)),
            Modifier::RelSize => Box::new(Dyn::Set(RelSize(self.value.parse()?))),
            Modifier::Content => Box::new(Dyn::Set(Content(self.value.to_owned()))),
        })
    }
    fn modifier(&self) -> Result<(TypeId, DynModifier)> {
        let type_id = self.modifier.type_id();
        let modifier = match self.flow {
            Flow::Dynamic => Box::new(Dyn::Ref::<()> { name: self.value.to_owned() }),
            Flow::Static => self.parse_modifier()?,
        };
        Ok((type_id, modifier))
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("A section with an opening brace is never closed.")]
    UnendingCloseSection,
    #[error("{0}")]
    ColorParse(#[from] color::Error),
    #[error("{0}")]
    FloatParse(#[from] ParseFloatError),
    #[error("A variable wasn't in the form of a keyword")]
    NonKeywordContentShorthand,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
struct Input<'a>(&'a str);

fn open_section_txt<'a>(input: &mut Input<'a>) -> &'a str {
    let full_input = input.0;
    loop {
        let double_newstart = input.0.starts_with("{{");
        let newstart = input.0.starts_with('{') && !double_newstart;
        match () {
            () if input.0.is_empty() => return full_input,
            () if newstart => return &full_input[0..full_input.len() - input.0.len()],
            () if double_newstart => input.0 = &input.0[2..],
            () => input.0 = &input.0[1..],
        }
    }
}
fn close_section_txt<'a>(input: &mut Input<'a>) -> Result<&'a str> {
    let full_input = input.0;
    loop {
        let double_close = input.0.starts_with("}}");
        let close = input.0.starts_with('}') && !double_close;
        match () {
            () if input.0.is_empty() => return Err(Error::UnendingCloseSection),
            () if close => {
                input.0 = &input.0[1..];
                let section_len = full_input.len() - input.0.len();
                // Without enclosing braces.
                return Ok(&full_input[1..section_len - 2]);
            }
            () if double_close => input.0 = &input.0[2..],
            () => input.0 = &input.0[1..],
        }
    }
}
/// Consumes a section and returns it as a `&str`.
///
/// A section can either be open or close:
/// - **open**: [`open_section_txt`]. Just plain text that cannot be modified
///   at runtime.
/// - **close**: [`close_section_txt`]. Delimited by opening and closing braces
///   `{` and `}`.
///
/// Note that braces are escaped similarly to rust's [`fmt`] format, by
/// repeating them: `{{` and `}}`.
fn section_txt<'a>(input: &mut Input<'a>) -> Result<(ContentType, &'a str)> {
    let curly_braced = input.0.starts_with('{') && !input.0.starts_with("{{");

    if curly_braced {
        close_section_txt(input).map(|t| (ContentType::Closed, t))
    } else {
        Ok((ContentType::Open, open_section_txt(input)))
    }
}

fn value_txt<'a>(input: &mut Input<'a>) -> Result<(Flow, &'a str)> {
    // TODO: check that indeed keyword.
    let (mut value, remaining) = input.0.split_once(',').unwrap_or((input.0, ""));
    input.0 = remaining;
    let flow = if value.ends_with('$') { Flow::Dynamic } else { Flow::Static };
    if let Flow::Dynamic = flow {
        value = &value[..value.len() - 1];
    }
    Ok((flow, value))
}
fn modifier(input: &mut Input) -> Option<Modifier> {
    let check_prefix = |input: &mut Input, prefix: &'static str, modifier| {
        let unprefixed = input.0.strip_prefix(prefix)?;
        input.0 = unprefixed;
        Some(modifier)
    };
    check_prefix(input, "color:", Modifier::Color)
        .or_else(|| check_prefix(input, "font:", Modifier::Font))
        .or_else(|| check_prefix(input, "size:", Modifier::RelSize))
        .or_else(|| check_prefix(input, "content:", Modifier::Content))
}

/// Parses a section element, consuming it from `input`.
///
/// An element is a key/value pair. Key being separated from a value
/// by a `:`, and elements separated from each other with a `,`.
///
/// The key is one of:
/// - `color:`
/// - `font:`
/// - `size:`
/// - `content:`
///
/// As of now, they do not support whitespaces.
fn element<'a>(is_first: bool, input: &mut Input<'a>) -> Result<Option<Element<'a>>> {
    let modifier = match modifier(input) {
        Some(modifier) => modifier,
        None if input.0.is_empty() => return Ok(None),
        None if is_first => return dynamic_content_shorthand(input),
        None => return static_content_shorthand(input),
    };
    let (flow, value) = value_txt(input)?;
    Ok(Some(Element { modifier, flow, value }))
}

fn static_content_shorthand<'a>(input: &mut Input<'a>) -> Result<Option<Element<'a>>> {
    let value = input.0;
    input.0 = "";

    Ok(Some(Element {
        modifier: Modifier::Content,
        flow: Flow::Static,
        value,
    }))
}

fn dynamic_content_shorthand<'a>(input: &mut Input<'a>) -> Result<Option<Element<'a>>> {
    let value = input.0;
    input.0 = "";

    if is_keyword(value) {
        Ok(Some(Element {
            modifier: Modifier::Content,
            flow: Flow::Dynamic,
            value,
        }))
    } else {
        Err(Error::NonKeywordContentShorthand)
    }
}

fn is_keyword(value: &str) -> bool {
    !value.is_empty() && value.chars().all(char::is_alphanumeric)
}

/// Parses a section, consuming it from `input`.
///
/// A section can either be open or close:
/// - **open**: [`open_section_txt`]. Just plain text that cannot be modified
///   at runtime.
/// - **close**: [`close_section_txt`]. Delimited by opening and closing braces
///   `{` and `}`. May contain [`element`]s, pairs of [`modifier`] and [`value_txt`].
fn section(input: &mut Input) -> Result<Option<Section>> {
    if input.0.is_empty() {
        return Ok(None);
    }
    let (closeness, section) = section_txt(input)?;

    if let ContentType::Open = closeness {
        return Ok(Some(Section {
            modifiers: {
                let mut ret = Modifiers::new();
                ret.insert(TypeId::of::<Content>(), Box::new(Content::from(section)));
                ret
            },
        }));
    }
    let mut section = Input(section);
    let mut modifiers = Modifiers::new();
    let mut is_first = true;
    loop {
        match element(is_first, &mut section)? {
            Some(element) => {
                let (type_id, modifier) = element.modifier()?;
                modifiers.insert(type_id, modifier);
            }
            None => return Ok(Some(Section { modifiers })),
        }
        is_first = false;
    }
}
pub(super) fn rich_text(input: &str) -> Result<Vec<Section>> {
    let mut sections = Vec::new();
    let mut input = Input(input);

    loop {
        match section(&mut input)? {
            Some(section) => sections.push(section),
            None => return Ok(sections),
        }
    }
}

// FIXME: Write lot of tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value() {
        todo!();
    }
    #[test]
    fn style_type() {
        todo!();
    }
    #[test]
    fn element() {
        todo!();
    }
    #[test]
    fn section() {
        todo!();
    }
    #[test]
    fn rich_text() {
        todo!();
    }
}
