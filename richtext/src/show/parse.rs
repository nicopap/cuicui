use std::{fmt, str::FromStr};

use winnow::{
    ascii::digit1, combinator::opt, combinator::preceded, sequence::delimited, token::one_of,
    Parser,
};

use super::RuntimeFormat;

fn parse<T: FromStr<Err = E>, E: fmt::Debug>(input: &str) -> T {
    input.parse().unwrap()
}

/// Parse a rust format specifier.
///
/// Refer to <https://doc.rust-lang.org/stable/std/fmt/index.html#syntax>.
/// Note that we really are only interested in `format_spec`, since supposedly
/// the rest of this crate does what `maybe_format` does, just much better for
/// our specific use-case.
pub(super) fn format(input: &str) -> Option<RuntimeFormat> {
    let fill = one_of::<&str, _, ()>(('a'..='z', 'A'..='Z', '0'..='9', ' '));
    let align = one_of("<^>");
    let sign = one_of("+-");

    let integer = || digit1.map(parse::<usize, _>);
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
    // TODO(err): discarding error here seems user-unfriendly
    delimited('{', opt(preceded(':', format_spec)), '}')
        .parse(input)
        .ok()
        .flatten()
}
