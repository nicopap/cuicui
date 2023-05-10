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

use crate::binding::Modifiers;
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
    mut ctx: interpret::Context,
    input: &str,
    trackers: &mut Vec<Tracker>,
) -> AnyResult<Modifiers> {
    let parsed = sections_inner.parse(input).map_err(|e| e.into_owned())?;

    let mut modifiers = Vec::with_capacity(parsed.0.len() * 2);

    for (i, sec) in parsed.0.into_iter().enumerate() {
        interpret::section(i, sec, &mut ctx, trackers, &mut modifiers)?;
    }
    Ok(Modifiers(modifiers.into()))
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use pretty_assertions::assert_eq;
    use winnow::error::ParseError;
    use winnow::Parser;

    use super::{
        balanced_text, bare_content, close_section, closed_element, sections_inner, structs,
    };

    macro_rules! sections {
        (@modifier {$binding:ident}) => {
            structs::Modifier {
                name: "Content",
                value: structs::Dyn::Dynamic(structs::Binding::Name(stringify!($binding))),
                subsection_count: 1,
            }
        };
        (@modifier $value:literal) => {
            structs::Modifier {
                name: "Content",
                value: structs::Dyn::Static($value),
                subsection_count: 1,
            }
        };
        (@modifier ( $name:ident $subsection_count:literal static $value:literal )) => {
            structs::Modifier {
                name: stringify!($name),
                value: structs::Dyn::Static($value),
                subsection_count: $subsection_count,
            }
        };
        (@modifier ( $name:ident $subsection_count:literal { $binding:ident } )) => {
            structs::Modifier {
                name: stringify!($name),
                value: structs::Dyn::Dynamic(structs::Binding::Name(stringify!($binding))),
                subsection_count: $subsection_count,
            }
        };
        (@section {$binding:ident}) => {
            structs::Section { modifiers: vec![ sections!(@modifier (Content 1 {$binding}) ) ] }
        };
        (@section $plain:literal) => {
            structs::Section { modifiers: vec![ sections!(@modifier $plain) ] }
        };
        (@section [ $( $modifier:tt ),* $(,)? ]) => {
            structs::Section {
                modifiers: vec![ $( sections!(@modifier $modifier) ),* ],
            }
        };
        ( $( $section:tt ),* $(,)? ) => {
            vec![ $( sections!(@section $section) ),* ]
        }
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
            "foo{x}",
            "foo{x}baz",
            "{x}baz",
            "{x}foo{x}",
            "{x}{x}",
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
            assert_eq!(parse(input), Ok(*remaining));
        }
    }

    #[test]
    fn closed_element_complete() {
        let parse = |input| parse_fn(closed_element, input);
        let complete = [
            "color:red",
            "color:{foobar}",
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
        let incomplete = [
            ("color:green, fancy", ", fancy"),
            ("content: foo | fancy", "| fancy"),
        ];
        for (input, remaining) in &incomplete {
            assert_eq!(parse(input), Ok(*remaining));
        }
    }
    #[test]
    fn closed_complete() {
        let parse = |input| parse_fn(close_section, input);
        let complete = [
            "{color: blue}",
            "{some_dynamic_content}",
            "{  color: blue  }",
            "{color: {fnoo}}",
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
    fn parse(input: &str) -> Result<Vec<structs::Section>, String> {
        sections_inner
            .parse(input)
            .map(|s| s.0)
            .map_err(|err| err.to_string())
    }
    #[test]
    fn plain_text() {
        let input = "This is some text, it is just a single content section";
        let expected = sections!["This is some text, it is just a single content section"];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn single_dynamic_shorthand() {
        let input = "This one contains a single {dynamic_content} that can be replaced at runtime";
        let expected = sections![
            "This one contains a single ",
            { dynamic_content },
            " that can be replaced at runtime",
        ];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn single_no_escape_closed_content() {
        let input = "{Color:white|This just has a single metadata}";
        let expected = sections![[
            "This just has a single metadata",
            (Color 1 static "white"),
        ]];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn single_closed_content() {
        let input =
            "{Color:white|This is also just some non-dynamic text, commas need not be escaped}";
        let expected = sections![[
            "This is also just some non-dynamic text, commas need not be escaped",
            (Color 1 static "white"),
        ]];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn closed_explicit_content_escape_comma() {
        let input = r#"{Content: This may also work\, but commas need to be escaped}"#;
        let expected = sections![r#"This may also work\, but commas need to be escaped"#];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn outer_dynamic_shorthand() {
        let input = "{dynamic_content}";
        let expected = sections![{ dynamic_content }];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn outer_color_mod() {
        let input = "{Color: Blue | This text is blue}";
        let expected = sections![
            [ "This text is blue",  (Color 1 static "Blue ") ]
        ];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn nested_dynamic_shorthand() {
        let input = "{Color: Blue | {dynamic_blue_content}}";
        let expected = sections![[
            {dynamic_blue_content},
            (Color 1 static "Blue "),
        ]];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn deep_nesting() {
        let input = "{Color: Blue | This is non-bold text: {Font:b|now it is bold, \
                you may also use {RelSize:1.3|{deeply_nested}} sections}, not anymore {Font:i|yet again}!}";
        let expected = sections![
            [ "This is non-bold text: " , (Color 7 static "Blue ") ],
            [ "now it is bold, you may also use " , (Font 3 static "b") ],
            [ {deeply_nested}, (RelSize 1 static "1.3") ],
            " sections",
            ", not anymore ",
            [ "yet again" , (Font 1 static "i") ],
            "!"
        ];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn multiple_mods() {
        let input =
            "{Color:Red| Some red text}, some default color {dynamic_name}. {Color:pink|And pink, why not?}";
        let expected = sections![
            [ "Some red text", (Color 1 static "Red") ],
            ", some default color ", {dynamic_name}, ". ",
            [ "And pink, why not?", (Color 1 static "pink") ],
        ];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn fancy_color_multiple_mods() {
        let input = "{Color:rgb(12, 34, 50),Font:bold.ttf|metadata values} can contain \
                commas within parenthesis or square brackets";
        let expected = sections![
            [ "metadata values", (Color 1 static "rgb(12, 34, 50)"), (Font 1 static "bold.ttf") ],
            " can contain commas within parenthesis or square brackets",
        ];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn escape_curlies_outer() {
        let input = r#"You can escape \{ curly brackets \}."#;
        let expected = sections![r#"You can escape \{ curly brackets \}."#];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn escape_backslash_outer() {
        let input = r#"Can also escape \\{Font:b|bold}"#;
        let expected = sections![
            r#"Can also escape \\"#,
            [ "bold" ,  (Font 1 static "b") ],
        ];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn escape_double_outer() {
        let input = r#"Can also escape \\\{}"#;
        let expected = sections![r#"Can also escape \\\{}"#,];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn escape_curlies_inner() {
        let input = r#"{Color: pink| even inside \{ a closed section \}}"#;
        let expected = sections![
            [ r#"even inside \{ a closed section \}"# ,  (Color 1 static "pink") ],
        ];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn escape_backslash_inner() {
        let input = r#"{Color: pink| This is \\ escaped}"#;
        let expected = sections![
            [ r#"This is \\ escaped"#,  (Color 1 static "pink") ],
        ];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn named_dynamic_mod() {
        let input = "{Color: {relevant_color} | Not only content can be dynamic, also value of other metadata}";
        let expected = sections![
            [ "Not only content can be dynamic, also value of other metadata" ,  (Color 1 {relevant_color}) ],
        ];
        assert_eq!(Ok(expected), parse(input));
    }
    #[test]
    fn all_dynamic_content_declarations() {
        let input =
            "{Content:{ident}} is equivalent to {ident} also {Color:white| {ident}} and {Color:white|{ident}}.";
        let expected =
            sections![
                {ident}, " is equivalent to ", {ident}, " also ",
                [ {ident}, (Color 1 static "white") ], " and ", [ {ident}, (Color 1 static "white") ], ".",
            ];
        assert_eq!(Ok(expected), parse(input));
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
            [
                (Font 1 static "fonts/FiraMono-Medium.ttf"),
                (Color 1 static "rgb(1.0, 0.5, 0.5)"),
                (RelSize 1 static "1.5"),
                {Score},
            ],
            "\n",
            [ (Color 1 static "rgb(1.0, 0.2, 0.2)"), {Deaths} ],
            "\nPaddle hits: ",
            [ (Color 1 static "pink"), {paddle_hits} ],
            "\nBall position: ",
            [ "\\{x: " ,  (Font 5 static "fonts/FiraMono-Medium.ttf"), (Color 5 static "pink") ],
            {ball_x}, ", y: ", {ball_y}, "\\}",
        ];
        assert_eq!(Ok(expected), parse(input));
    }
    // ---------------------------------
}
