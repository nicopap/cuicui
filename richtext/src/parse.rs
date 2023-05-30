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
mod post_process;
mod tree;

use winnow::{
    ascii::{alpha1, alphanumeric1, digit1, escaped, multispace0},
    branch::alt,
    combinator::{
        delimited, fail, opt, peek, preceded, repeat0, separated1, separated_pair, terminated,
    },
    dispatch,
    error::ParseError,
    stream::{AsChar, Stream, StreamIsPartial},
    token::{any, one_of, take_till0, take_till1},
    Parser,
};

use crate::show::RuntimeFormat;
use tree::{flatten_section, Binding, Dyn, Modifier, Path, Section, Sections};

pub(crate) use color::parse as color;
pub(crate) use post_process::{Repeat, Tree, TreeSplitter};
pub(crate) use tree::{Format, Hook, Query, Source};

type IResult<'a, O> = winnow::IResult<&'a str, O>;

// How to read the following code:
// Look at the variable names for match with the grammar linked in module doc,
// they are defined in the same order.

/// Parse the prefix of the `Target` string.
fn path(input: &str) -> IResult<Path> {
    let namespace = dispatch! {ident;
        "Res" => preceded('.', ident).map(Query::Res),
        "One" => delimited('(', ident, ')').map(Query::One),
        "Name" => (delimited('(', ident, ")."), ident).map(Query::name),
        "Marked" => (delimited('(', ident, ")."), ident).map(Query::marked),
        _ => fail,
    };
    let reflect_path = escaped(take_till0(":}\\"), '\\', one_of(":}\\"));
    let source = (namespace, reflect_path).with_recognized().map(Source::new);

    let mut path = alt((source.map(Path::Tracked), ident.map(Path::Binding)));
    path.parse_next(input)
}

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
    let format = (path, opt(preceded(':', format_spec))).map(Binding::format);
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
        .map(Sections::full_subsection)
        .parse_next(input)
}
fn sections(input: &str) -> IResult<Sections> {
    (open_section, repeat0((close_section, open_section)))
        .map(Sections::full_subsection)
        .parse_next(input)
}
pub(super) fn richtext(input: &str) -> anyhow::Result<Tree> {
    let sections = sections.parse(input).map_err(|e| e.into_owned())?;
    Ok(Tree::new(sections.0))
}
