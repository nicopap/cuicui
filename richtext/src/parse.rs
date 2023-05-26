//! Parse rich text according to spec
//!
//! See the grammar at <https://github.com/nicopap/cuicui/blob/main/design_doc/richtext/informal_grammar.md>
//! or the file at `design_doc/richtext/informal_grammar.md` from the root of
//! the git repository.
//!
//! The code in this module is little more than 1-to-1 implementation of the
//! grammar.

#[cfg(test)]
mod tests;

mod color;
mod error;
mod structs;

use std::borrow::Cow;

use fab::{binding, resolve::MakeModify as FabModifier};
use winnow::{
    ascii::{alpha1, alphanumeric1, digit1, escaped, multispace0},
    branch::alt,
    combinator::{
        delimited, fail, opt, peek, preceded, repeat0, separated1, separated_pair, terminated,
    },
    dispatch,
    error::ParseError,
    stream::{AsChar, Stream, StreamIsPartial},
    token::{any, one_of, take_till1, take_while1},
    Parser,
};

use crate::{modifiers::TextModifiers, richtext::TextPrefab, show::RuntimeFormat, track::Tracker};
use structs::{flatten_section, Binding, Dyn, Format, Modifier, Section, Sections};

pub(crate) use color::parse as color;

type IResult<'a, O> = winnow::IResult<&'a str, O>;
type AnyResult<T> = anyhow::Result<T>;

// How to read the following code:
// Look at the variable names for match with the grammar linked in module doc,
// they are defined in the same order.

/// Parse a rust format specifier.
///
/// Refer to <https://doc.rust-lang.org/stable/std/fmt/index.html#syntax>.
/// Note that we really are only interested in `format_spec`, since supposedly
/// the rest of this crate does what `maybe_format` does, just much better for
/// our specific use-case.
fn binding(input: &str) -> IResult<Binding> {
    let fill = one_of::<&str, _, _>(('a'..='z', 'A'..='Z', '0'..='9', ' '));
    let align = one_of("<^>");
    let sign = one_of("+-");

    let integer = || digit1.map(|i: &str| i.parse().unwrap());
    let count = integer;
    let width = count();
    let precision = count();

    let type_ = opt('?');

    let format_spec = (
        opt((opt(fill), align)),
        opt(sign),
        opt('#'),
        opt('0'),
        opt(width),
        opt(preceded('.', precision)),
        type_,
    );
    let format_spec = format_spec.map(|(_, sign, _, _, width, prec, type_)| RuntimeFormat {
        sign: sign.is_some(),
        width: width.unwrap_or(0),
        prec: prec.unwrap_or(0),
        debug: type_.is_some(),
    });
    let format_spec = alt((ident.map(Format::UserDefined), format_spec.map(Format::Fmt)));
    let path = take_while1(('a'..='z', 'A'..='Z', '0'..='9', "_."));
    let format = separated_pair(path, ':', format_spec);
    let format = alt((format.map(Binding::format), ident.map(Binding::Name)));
    terminated(format, peek('}'))
        .context("format")
        .parse_next(input)
}

fn ws<I, O, E>(inner: impl Parser<I, O, E>) -> impl Parser<I, O, E>
where
    <I as Stream>::Token: AsChar,
    <I as Stream>::Token: Copy,
    I: StreamIsPartial + Stream,
    E: ParseError<I>,
{
    delimited(multispace0, inner, multispace0)
}

fn ident(input: &str) -> IResult<&str> {
    let repeat = repeat0::<_, _, (), _, _>;
    (alt((alpha1, "_")), repeat(alt((alphanumeric1, "_"))))
        .recognize()
        .parse_next(input)
}
fn balanced_text(input: &str) -> IResult<&str> {
    fn scope(input: &str) -> IResult<&str> {
        let semi_exposed = || escaped(take_till1("()[]{}\\"), '\\', one_of("()[]{}|,\\"));
        let repeat = repeat0::<_, _, (), _, _>;
        let inner = move || (semi_exposed(), repeat((scope, semi_exposed())));
        let dispatch = dispatch! {any;
            '{' => terminated(inner(), '}'),
            '[' => terminated(inner(), ']'),
            '(' => terminated(inner(), ')'),
            _ => fail,
        };
        dispatch.recognize().parse_next(input)
    }
    let exposed = || escaped(take_till1("([{}|,\\"), '\\', one_of("()[]{}|,\\"));

    let repeat = repeat0::<_, _, (), _, _>;
    (exposed(), repeat((scope, exposed())))
        .recognize()
        .parse_next(input)
}
fn open_subsection(input: &str) -> IResult<Option<Section>> {
    escaped(take_till1("{}\\"), '\\', one_of("{}\\"))
        .map(Section::free)
        .parse_next(input)
}
fn open_section(input: &str) -> IResult<Option<Section>> {
    escaped(take_till1("{\\"), '\\', one_of("{}\\"))
        .map(Section::free)
        .parse_next(input)
}
fn close_section(input: &str) -> IResult<Vec<Section>> {
    let full_list = (
        separated1(closed_element, ws(',')),
        opt(preceded(ws('|'), bare_content)),
    );
    let closed = alt((
        binding.map(Section::format),
        full_list.map(flatten_section).context("meta list"),
    ));
    delimited('{', ws(closed), '}').parse_next(input)
}
fn closed_element(input: &str) -> IResult<Modifier> {
    let key = ident.context("key");

    let metadata = alt((
        delimited('{', binding, '}').map(Dyn::Dynamic),
        balanced_text.context("metadata value").map(Dyn::Static),
    ));
    separated_pair(key, ws(':'), metadata)
        .map(Modifier::new)
        .parse_next(input)
}
fn bare_content(input: &str) -> IResult<Sections> {
    let open_sub = open_subsection;
    (open_sub, repeat0((close_section, open_sub)))
        .context("section content")
        .map(Sections::tail)
        .parse_next(input)
}
fn sections_inner(input: &str) -> IResult<Sections> {
    (open_section, repeat0((close_section, open_section)))
        .map(Sections::tail)
        .parse_next(input)
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
pub(super) fn richtext(
    bindings: &mut binding::World<TextPrefab>,
    input: &str,
    trackers: &mut Vec<Tracker>,
) -> AnyResult<Vec<FabModifier<TextPrefab>>> {
    use crate::track::make_tracker;
    use fab::resolve::ModifyKind;
    macro_rules! bound {
        ($name:expr) => {
            Ok(ModifyKind::Bound(bindings.get_or_add($name)))
        };
    }
    let parsed = sections_inner.parse(input).map_err(|e| e.into_owned())?;
    parsed
        .0
        .into_iter()
        .enumerate()
        .flat_map(|(i, sec)| {
            sec.modifiers
                .into_iter()
                // NOTE: `ParseModifier` are sorted in ascending order (most specific
                // to most general), but we want the reverse, most general to most specific,
                // so that, when we iterate through it, we can apply general, the specific etc.
                .rev()
                .map(|Modifier { name, value, subsection_count }| {
                    let try_u32 = u32::try_from;
                    let kind = match value {
                        Dyn::Dynamic(Binding::Name(name)) => bound!(name),
                        Dyn::Dynamic(Binding::Format { path, format }) => {
                            let show = match format {
                                Format::Fmt(format) => Box::new(format),
                                Format::UserDefined(_) => {
                                    todo!("TODO(feat): user-specified formatters")
                                }
                            };
                            // TODO(err): unwrap
                            let tracker = make_tracker(path.to_string(), path, show).unwrap();
                            trackers.push(tracker);
                            bound!(path)
                        }
                        Dyn::Static(value) => {
                            let mut value: Cow<'static, str> = value.to_owned().into();
                            escape_backslashes(&mut value);
                            TextModifiers::parse(name, &value).map(ModifyKind::Modify)
                        }
                    };
                    let modifier = FabModifier {
                        kind: kind?,
                        range: try_u32(i)?..try_u32(i + subsection_count)?,
                    };
                    Ok(modifier)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
