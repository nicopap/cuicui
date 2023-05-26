use winnow::{
    ascii::{alpha1, alphanumeric1, escaped},
    branch::alt,
    combinator::{delimited, preceded, repeat0},
    error::Error,
    token::{one_of, take_till0},
    Parser,
};

// Target grammar:
//
// ```
// prefix =
//     'Res' '.' ident
//     | 'One' '(' ident ')'
//     | 'Name' '(' ident ')' '.' ident
//     | 'Marked' '(' ident ')' '.' ident
// path = <text∌:>
// target = prefix path
// ```
use super::{RefNamespace, RefTarget};

type IResult<'a, O> = winnow::IResult<&'a str, O>;

fn ident(input: &str) -> IResult<&str> {
    let repeat = repeat0::<_, _, (), _, _>;
    (alt((alpha1, "_")), repeat(alt((alphanumeric1, "_"))))
        .recognize()
        .parse_next(input)
}

/// Parse the prefix of the `Target` string.
pub(super) fn target(input: &str) -> Result<RefTarget, Error<&str>> {
    use RefNamespace::{Marked, Name, One, Res};
    let prefix = alt((
        preceded("Res.", ident).map(Res),
        preceded("One", delimited('(', ident, ')')).map(One),
        preceded("Name", (delimited('(', ident, ")."), ident))
            .map(|(name, access)| Name { name, access }),
        preceded("Marked", (delimited('(', ident, ")."), ident))
            .map(|(marker, access)| Marked { marker, access }),
    ));
    let path = escaped(take_till0(":\\"), '\\', one_of(":\\"));
    (prefix, path)
        .map(|(namespace, path)| RefTarget { namespace, path })
        .parse(input)
}
