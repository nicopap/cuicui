//! Parse rich text according to spec
//!
//! See the grammar at <https://github.com/nicopap/cuicui/blob/main/design_doc/richtext/informal_grammar.md>
//! or the file at `design_doc/richtext/informal_grammar.md` from the root of
//! the git repository.
//!
//! The code in this module is little more than 1-to-1 implementation of the
//! grammar.

mod color;
mod error;
pub(crate) mod interpret;
mod structs;

use winnow::{
    ascii::{alpha1, alphanumeric1, digit1, escaped, multispace0},
    branch::alt,
    combinator::{delimited, opt, peek, preceded, repeat0, separated1, separated_pair, terminated},
    error::ParseError,
    stream::{AsChar, Stream, StreamIsPartial},
    token::{one_of, take_till1, take_while1},
    Parser,
};

use crate::AnyError;
use crate::{show::RuntimeFormat, track::Tracker};
use structs::{flatten_section, Binding, Dyn, Format, Modifier, Section, Sections};

pub(crate) use color::parse as color;

type IResult<'a, O> = winnow::IResult<&'a str, O>;
type AnyResult<T> = Result<T, AnyError>;

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
    let format = alt((
        peek('}').map(|_| Binding::Type),
        preceded(("fmt:", multispace0), format).map(Binding::format),
        terminated(ident, peek('}')).map(Binding::Name),
    ));
    format.context("format").parse_next(input)
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
        let inner = || (semi_exposed(), repeat((scope, semi_exposed())));
        // TODO(perf): this is slow, need to replace with `dispatch!`
        alt((
            delimited('{', inner(), '}'),
            delimited('[', inner(), ']'),
            delimited('(', inner(), ')'),
        ))
        .recognize()
        .parse_next(input)
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
pub(super) fn richtext(
    ctx: interpret::Context,
    input: &str,
    trackers: &mut Vec<Tracker>,
) -> AnyResult<Vec<crate::Section>> {
    let parsed = sections_inner.parse(input).map_err(|e| e.into_owned())?;
    let parsed = parsed.0.into_iter();
    parsed
        .map(|s| interpret::section(s, &ctx, trackers))
        .collect::<Result<_, _>>()
}

#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};
    use std::fmt;

    use bevy::prelude::Color as Col;
    use pretty_assertions_sorted::assert_eq_sorted;
    use winnow::error::ParseError;
    use winnow::Parser;

    use super::super::{modifiers, Modifiers};
    use super::{balanced_text, bare_content, close_section, closed_element, interpret, richtext};

    use crate::Section;

    macro_rules! sections {
        (@type_id $actual:ident $($_:tt)*) => {
                TypeId::of::<modifiers::$actual>()
        };
        (@modifiers $( $((fn $id:ident))? $($modifier:ident)::* ( $value:expr ) ),* $(,)? ) => {{
            let mut modifiers = Modifiers::default();
            $(
                let id = sections!(@type_id $($id)? $($modifier)*);
                let value = modifiers::$($modifier)::*( $value );
                modifiers.insert(id, Box::new(value));
            )*
            Section { modifiers }
        }};
        (@item $text:literal) => {
            sections!(@modifiers Content($text.to_owned().into()))
        };
        (@item {$( $(( fn $type_id:ident ))? $($modifier:ident )::* : $value:expr ),* $(,)? }) => {
            sections!(@modifiers $( $((fn $type_id))? $($modifier)::*($value) ),*)
        };
        ($( $item:tt ),* $(,)?) => {
            vec![ $( sections!(@item $item) ),* ]
        }
    }
    fn s<'a, T: From<&'a str>>(input: &'a str) -> T {
        input.into()
    }
    fn id<T: Any>() -> TypeId {
        TypeId::of::<T>()
    }
    fn parse_fn<'a, T, E: fmt::Display + fmt::Debug + ParseError<&'a str>>(
        parser: impl Parser<&'a str, T, E>,
        input: &'a str,
    ) -> Result<T, String> {
        let mut parser = winnow::trace::trace(format!("TRACE_ROOT \"{input}\""), parser);
        parser.parse(input).map_err(|err| {
            println!("{err}");
            err.to_string()
        })
    }

    fn parse_bad<'a, T, E: fmt::Display + fmt::Debug>(
        parser: impl Parser<&'a str, T, E>,
        input: &'a str,
    ) -> Result<&'a str, String> {
        let mut parser = winnow::trace::trace(format!("TRACE_ROOT \"{input}\""), parser);
        parser.parse_next(input).map(|t| t.0).map_err(|err| {
            println!("{err}");
            err.to_string()
        })
    }
    #[test]
    fn bare_content_complete() {
        let parse = |input| parse_fn(bare_content, input);
        let complete = [
            "foo",
            r#"foo\}bar"#,
            "foo{}",
            "foo{}baz",
            "{}baz",
            "{}foo{}",
            "{}{}",
            "foo{content:test}bar",
            "foo{color:blue |green}bar",
            "foo{color:blue |hi{hello_world}hi}bar",
        ];
        for input in &complete {
            let output = parse(input);
            assert!(output.is_ok(), "{:?}", output);
        }
    }

    #[test]
    fn balanced_text_complete() {
        let parse = |input| parse_fn(balanced_text, input);
        let complete = [
            r#"foo \| bar"#,
            "",
            "foo bar",
            "foo (bar) baz",
            "foo (,) baz",
            "foo (bar) (baz) zab",
            "(foo)",
            "(foo , bar)",
            "(,)",
            "(foo (bar) baz)",
            "(foo () baz) bar",
            "(foo ()() baz) bar",
            "()",
            r#"food ( \| ) blar"#,
            "foo [] bar",
            "foo ({},[]) bar",
            r#"foo \, bar"#,
            "foo (|) bar",
            r#"(foo \{ bar)"#,
            r#"(foo \{ |)"#,
        ];
        for input in &complete {
            let output = parse(input);
            assert!(output.is_ok(), "{:?}", output);
        }
    }
    #[test]
    fn balanced_text_incomplete() {
        let parse = |input| parse_bad(balanced_text, input);
        let incomplete = [
            ("foo , bar", ", bar"),
            (",", ","),
            ("foo | bar", "| bar"),
            (r#"foo \, , bar"#, ", bar"),
            (r#"foo , \, bar"#, r#", \, bar"#),
            (r#"foo \( , \) bar"#, r#", \) bar"#),
        ];
        for (input, remaining) in &incomplete {
            assert_eq_sorted!(parse(input), Ok(*remaining));
        }
    }

    #[test]
    fn closed_element_complete() {
        let parse = |input| parse_fn(closed_element, input);
        let complete = [
            "color:red",
            "color:{foobar}",
            "color:{}",
            r#"content:   foo\,bar"#,
            "color: rgb(1,3,4)",
            "color:PiNk",
            "font: foo.ttf",
            "size: 6.28318530",
        ];
        for input in &complete {
            let output = parse(input);
            assert!(output.is_ok(), "{:?}", output);
        }
    }
    #[test]
    fn closed_element_incomplete() {
        let parse = |input| parse_bad(closed_element, input);
        // TODO(test): error on "", "mahagony: expensive", "kouglov", "darth Mouse:"
        let incomplete = [
            ("color:green, fancy", ", fancy"),
            ("content: foo | fancy", "| fancy"),
        ];
        for (input, remaining) in &incomplete {
            assert_eq_sorted!(parse(input), Ok(*remaining));
        }
    }
    #[test]
    fn closed_complete() {
        let parse = |input| parse_fn(close_section, input);
        let complete = [
            "{color: blue}",
            "{}",
            "{some_dynamic_content}",
            "{  color: blue  }",
            "{color: {fnoo}}",
            "{color  : {}}",
            "{color:{}}",
            "{color: purple, font: blar}",
            "{color: cyan, font: bar | dolore abdicum}",
            "{color:orange |lorem ipsum}",
        ];
        for input in &complete {
            let output = parse(input);
            assert!(output.is_ok(), "{:?}", output);
        }
    }
    #[test]
    fn closed_incomplete() {
        let _parse = |input| parse_bad(close_section, input);
    }

    // ---------------------------------
    //        test rich_text parsing
    // ---------------------------------
    fn parse(input: &str) -> Result<Vec<Section>, String> {
        let mut discard = Vec::new();
        richtext(interpret::Context::richtext_defaults(), input, &mut discard)
            .map_err(|err| err.to_string())
    }
    #[test]
    fn plain_text() {
        let input = "This is some text, it is just a single content section";
        let expected = sections!["This is some text, it is just a single content section"];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn single_dynamic_shorthand() {
        let input = "This one contains a single {dynamic_content} that can be replaced at runtime";
        let expected = sections![
            "This one contains a single ",
            {(fn Content) Dynamic::new : s("dynamic_content")},
            " that can be replaced at runtime",
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn single_no_escape_closed_content() {
        let input = "{Color:white|This just has a single metadata}";
        let expected = sections![{
            Color: Col::WHITE,
            Content: s("This just has a single metadata"),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn single_closed_content() {
        let input =
            "{Color:white|This is also just some non-dynamic text, commas need not be escaped}";
        let expected = sections![{
            Color: Col::WHITE,
            Content: s("This is also just some non-dynamic text, commas need not be escaped"),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn closed_explicit_content_escape_comma() {
        let input = r#"{Content: This may also work\, but commas need to be escaped}"#;
        let expected = sections!["This may also work, but commas need to be escaped"];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn outer_dynamic_shorthand() {
        let input = "{dynamic_content}";
        let expected = sections![{(fn Content) Dynamic::new : s("dynamic_content")}];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn outer_dynamic_content_implicit() {
        let input = "{}";
        let expected = sections![{(fn Content) Dynamic::ByType : id::<modifiers::Content>()}];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn dynamic_content_implicit() {
        let input = "An empty {} is equivalent to {name}, but referred by typeid instead of name";
        let expected = sections![
            "An empty ",
            {(fn Content) Dynamic::ByType : id::<modifiers::Content>()},
            " is equivalent to ",
            {(fn Content) Dynamic::new : s("name")},
            ", but referred by typeid instead of name"
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn outer_color_mod() {
        let input = "{Color: Blue | This text is blue}";
        let expected = sections![
            {Color: Col::BLUE, Content: s("This text is blue")},
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn nested_dynamic_shorthand() {
        let input = "{Color: Blue | {dynamic_blue_content}}";
        let expected = sections![{
            Color: Col::BLUE,
            (fn Content) Dynamic::new: s("dynamic_blue_content"),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn deep_nesting() {
        let input = "{Color: Blue | This is non-bold text: {Font:b|now it is bold, \
                you may also use {RelSize:1.3|{deeply_nested}} sections}, not anymore {Font:i|yet again}!}";
        let expected = sections![
            { Color: Col::BLUE, Content: s("This is non-bold text: ") },
            { Color: Col::BLUE, Font: s("b"), Content: s("now it is bold, you may also use ") },
            { Color: Col::BLUE, Font: s("b"), RelSize: 1.3, (fn Content) Dynamic::new: s("deeply_nested") },
            { Color: Col::BLUE, Font: s("b"), Content: s(" sections") },
            { Color: Col::BLUE, Content: s(", not anymore ") },
            { Color: Col::BLUE, Font: s("i"), Content: s("yet again") },
            { Color: Col::BLUE, Content: s("!") },
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn multiple_mods() {
        let input =
            "{Color:Red| Some red text}, some default color {dynamic_name}. {Color:pink|And pink, why not?}";
        let expected = sections![
            { Color: Col::RED, Content: s("Some red text") },
            ", some default color ",
            { (fn Content) Dynamic::new: s("dynamic_name") },
            ". ",
            { Color: Col::PINK, Content: s("And pink, why not?") },
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn fancy_color_multiple_mods() {
        let input = "{Color:rgb(12, 34, 50),Font:bold.ttf|metadata values} can contain \
                commas within parenthesis or square brackets";
        let expected = sections![
            { Color: Col::rgb_u8(12,34,50), Font: s("bold.ttf"), Content: s("metadata values") },
            " can contain commas within parenthesis or square brackets",
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn escape_curlies_outer() {
        let input = r#"You can escape \{ curly brackets \}."#;
        let expected = sections!["You can escape { curly brackets }."];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn escape_backslash_outer() {
        let input = r#"Can also escape \\{Font:b|bold}"#;
        let expected = sections![
            r#"Can also escape \"#,
            { Font: s("b"), Content: s("bold") },
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn escape_double_outer() {
        let input = r#"Can also escape \\\{}"#;
        let expected = sections![r#"Can also escape \{}"#,];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn escape_curlies_inner() {
        let input = r#"{Color: pink| even inside \{ a closed section \}}"#;
        let expected = sections![{
            Color: Col::PINK,
            Content: s("even inside { a closed section }"),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn escape_backslash_inner() {
        let input = r#"{Color: pink| This is \\ escaped}"#;
        let expected = sections![{
            Color: Col::PINK,
            Content: s(r#"This is \ escaped"#),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn named_dynamic_mod() {
        let input = "{Color: {relevant_color} | Not only content can be dynamic, also value of other metadata}";
        let expected = sections![{
            (fn Color) Dynamic::new: s("relevant_color"),
            Content: s("Not only content can be dynamic, also value of other metadata"),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn implicit_dynamic_mod() {
        let input = "{Color: {} |If the identifier of a dynamic metadata value is elided, \
                then the typeid of the rust type is used}";
        let expected = sections![{
            (fn Color) Dynamic::ByType: id::<modifiers::Color>(),
            Content: s(
                "If the identifier of a dynamic metadata value is elided, \
                then the typeid of the rust type is used"
            ),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn implicit_dynamic_content_mod() {
        let input = "can also use a single elided content if you want: {Content:{}}";
        let expected = sections![
            "can also use a single elided content if you want: ",
            {(fn Content) Dynamic::ByType: id::<modifiers::Content>()},
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn all_dynamic_content_declarations() {
        let input =
            "{Content:{ident}} is equivalent to {ident} also {Color:white| {ident}} and {Color:white|{ident}}.";
        let expected = sections![
            {(fn Content) Dynamic::new: s("ident")},
            " is equivalent to ",
            {(fn Content) Dynamic::new: s("ident")},
            " also ",
            {Color: Col::WHITE, (fn Content) Dynamic::new: s("ident")},
            " and ",
            {Color: Col::WHITE, (fn Content) Dynamic::new: s("ident")},
            ".",
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn real_world_usecase() {
        let input =
            "Score: {Font: fonts/FiraMono-Medium.ttf, Color: rgb(1.0, 0.5, 0.5), RelSize: 1.5, Content: {Score}}\n\
            {Color: rgb(1.0, 0.2, 0.2), Content: {Deaths}}\n\
            Paddle hits: {Color: pink, Content: {paddle_hits}}\n\
            Ball position: {Font: fonts/FiraMono-Medium.ttf, Color: pink|\\{x: {ball_x}, y: {ball_y}\\}}";
        let expected = sections![
            "Score: ",
            {(fn Content) Dynamic::new: s("Score"), Color: Col::rgb(1.0,0.5,0.5), Font: s("fonts/FiraMono-Medium.ttf"), RelSize: 1.5},
            "\n",
            {(fn Content) Dynamic::new: s("Deaths"), Color: Col::rgb(1.0,0.2,0.2)},
            "\nPaddle hits: ",
            {(fn Content) Dynamic::new: s("paddle_hits"), Color: Col::PINK},
            "\nBall position: ",
            {Color: Col::PINK, Font: s("fonts/FiraMono-Medium.ttf"), Content: s("{x: ")},
            {Color: Col::PINK, Font: s("fonts/FiraMono-Medium.ttf"), (fn Content) Dynamic::new: s("ball_x")},
            {Color: Col::PINK, Font: s("fonts/FiraMono-Medium.ttf"), Content: s(", y: ")},
            {Color: Col::PINK, Font: s("fonts/FiraMono-Medium.ttf"), (fn Content) Dynamic::new: s("ball_y")},
            {Color: Col::PINK, Font: s("fonts/FiraMono-Medium.ttf"), Content: s("}")},
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    // ---------------------------------
}
