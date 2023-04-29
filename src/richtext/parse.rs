//! Parse rich text according to spec
mod helpers;

use thiserror::Error;
use winnow::{
    ascii::{alpha1, alphanumeric1, escaped},
    branch::alt,
    combinator::{delimited, opt, preceded, repeat0, separated1, separated_pair},
    token::{one_of, take_till1},
    IResult, Parser,
};

use super::{RichText, Section};
use helpers::{elements_and_content, short_dynamic, ws, Element, ModifierValue, Sections};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    WinnowBad(#[from] winnow::error::Error<String>),
    #[error("trailing content: {0}")]
    Trailing(String),
}
impl From<winnow::error::Error<&'_ str>> for Error {
    fn from(value: winnow::error::Error<&'_ str>) -> Self {
        Self::WinnowBad(value.into_owned())
    }
}

// ```
// <ident>: "identifier respecting rust's identifier rules"
// <text∌FOO>: "text that doesn't contain FOO, unless prefixed by backslash `\`
//              may be empty"
// scope = '{' inner '}' | '(' inner ')' | '[' inner ']'
// semi_exposed = <text∌()[]{}>
// inner = semi_exposed [scope semi_exposed]*
// exposed = <text∌([{}|,>
// balanced_text = exposed [scope exposed]*
//
// key = <ident>
// open_subsection = <text∌{}>
// open_section = <text∌{>
// close_section = '{' closed '}'
// closed_element = key ':' metadata
// closed = <ident> | [closed_element],* ['|' bare_content]?
// metadata = '$' [<ident>]? | balanced_text
// bare_content = open_subsection [close_section open_subsection]*
// rich_text = open_section [close_section open_section]*
// ```
//
// How to read the following code:
// Look at the variable names for match with the grammar, they are defined in the same order.
fn ident(input: &str) -> IResult<&str, &str> {
    let repeat = repeat0::<_, _, (), _, _>;
    (alt((alpha1, "_")), repeat(alt((alphanumeric1, "_"))))
        .recognize()
        .context("ident")
        .parse_next(input)
}
fn balanced_text(input: &str) -> IResult<&str, &str> {
    fn scope(input: &str) -> IResult<&str, &str> {
        let semi_exposed = || escaped(take_till1("()[]{}\\"), '\\', one_of("()[]{}|,\\"));
        let repeat = repeat0::<_, _, (), _, _>;
        let inner = || (semi_exposed(), repeat((scope, semi_exposed())));
        // TODO(perf): this is slow, need to replace with `dispatch!`
        alt((
            delimited('{', inner(), '}'),
            delimited('[', inner(), ']'),
            delimited('(', inner(), ')'),
        ))
        .context("scope")
        .recognize()
        .parse_next(input)
    }
    let exposed = || escaped(take_till1("([{}|,\\"), '\\', one_of("()[]{}|,\\"));

    let repeat = repeat0::<_, _, (), _, _>;
    (exposed(), repeat((scope, exposed())))
        .context("balanced_text")
        .recognize()
        .parse_next(input)
}
fn open_subsection(input: &str) -> IResult<&str, Option<Section>> {
    escaped(take_till1("{}\\"), '\\', one_of("{}\\"))
        .map(Section::opt_from)
        .context("open_subsection")
        .parse_next(input)
}
fn open_section(input: &str) -> IResult<&str, Option<Section>> {
    escaped(take_till1("{\\"), '\\', one_of("{}\\"))
        .map(Section::opt_from)
        .context("open_section")
        .parse_next(input)
}
fn close_section(input: &str) -> IResult<&str, Vec<Section>> {
    let full_list = (
        separated1(closed_element, ws(',').context("comma")),
        opt(preceded(ws('|').context("bar"), bare_content)),
    );
    let closed = alt((
        // TODO(err): actually capture error instead of eating it in winnow
        full_list.map(|t| elements_and_content(t).unwrap()),
        opt(ident).map(short_dynamic),
    ));
    delimited('{', ws(closed.context("closed")), '}')
        .context("close_section")
        .parse_next(input)
}
fn closed_element(input: &str) -> IResult<&str, Element> {
    use ModifierValue as Mod;

    // TODO(feat): dynamic tags
    let key = ident.context("key");

    let metadata = alt((
        preceded('$', opt(ident)).context("$meta").map(Mod::dyn_opt),
        balanced_text.map(Mod::statik),
    ));
    separated_pair(key, ws(':'), metadata.context("metadata"))
        .context("closed_element")
        .map(Element::modifier)
        .parse_next(input)
}
fn bare_content(input: &str) -> IResult<&str, Sections> {
    let open = open_subsection;
    (open, repeat0((close_section, open)))
        .context("bare_content")
        .map(Sections::tail)
        .parse_next(input)
}
fn rich_text_inner(input: &str) -> IResult<&str, Sections> {
    (open_section, repeat0((close_section, open_section)))
        .context("rich_text")
        .map(Sections::tail)
        .parse_next(input)
}
pub(super) fn rich_text(input: &str) -> Result<RichText, winnow::error::Error<&str>> {
    rich_text_inner.map(RichText::from).parse(input)
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::fmt;

    use bevy::prelude::Color as Col;
    use pretty_assertions_sorted::assert_eq_sorted;
    use winnow::error::ParseError;
    use winnow::Parser;

    use super::super::{modifiers, Modifiers};
    use super::*;

    macro_rules! sections {
        (@type_id $actual:ident $($_:tt)*) => {
                TypeId::of::<modifiers::$actual>()
        };
        (@modifiers $( $((fn $id:ident))? $($modifier:ident)::* ( $value:expr ) ),* $(,)? ) => {{
            let mut modifiers = Modifiers::new();
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
            // ('(', ""),
            // ("foo ( bar", ""),
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
            "color:$foobar",
            "color:$",
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
            "{color: $fnoo}",
            "{color  : $}",
            "{color:$}",
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
        parse_fn(rich_text_inner, input).map(|rt| rt.0)
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
        let input = "{color:white|This just has a single metadata}";
        let expected = sections![{
            Color: Col::WHITE,
            Content: s("This just has a single metadata"),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn single_closed_content() {
        let input =
            "{color:white|This is also just some non-dynamic text, commas need not be escaped}";
        let expected = sections![{
            Color: Col::WHITE,
            Content: s("This is also just some non-dynamic text, commas need not be escaped"),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn closed_explicit_content_escape_comma() {
        let input = r#"{content: This may also work\, but commas need to be escaped}"#;
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
        let expected=
                // TODO(feat): this needs to be TypeId.of(Content)
                sections![{(fn Content) Dynamic::new : s("content")}];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn dynamic_content_implicit() {
        let input = "An empty {} is equivalent to {name}, but referred by typeid instead of name";
        let expected = sections![
            "An empty ",
            // TODO(feat): this needs to be TypeId.of(Content)
            {(fn Content) Dynamic::new : s("content")},
            " is equivalent to ",
            {(fn Content) Dynamic::new : s("name")},
            ", but referred by typeid instead of name"
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn outer_color_mod() {
        let input = "{color: Blue | This text is blue}";
        let expected = sections![
            {Color: Col::BLUE, Content: s("This text is blue")},
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn nested_dynamic_shorthand() {
        let input = "{color: Blue | {dynamic_blue_content}}";
        let expected = sections![{
            Color: Col::BLUE,
            (fn Content) Dynamic::new: s("dynamic_blue_content"),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn deep_nesting() {
        let input = "{color: Blue | This is non-bold text: {font:b|now it is bold, \
                you may also use {size:1.3|{deeply_nested}} sections}, not anymore {font:i|yet again}!}";
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
            "{color:Red| Some red text}, some default color {dynamic_name}. {color:pink|And pink, why not?}";
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
        let input = "{color:rgb(12, 34, 50),font:bold.ttf|metadata values} can contain \
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
        let input = r#"Can also escape \\{font:b|bold}"#;
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
        let input = r#"{color: pink| even inside \{ a closed section \}}"#;
        let expected = sections![{
            Color: Col::PINK,
            Content: s("even inside { a closed section }"),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn escape_backslash_inner() {
        let input = r#"{color: pink| This is \\ escaped}"#;
        let expected = sections![{
            Color: Col::PINK,
            Content: s(r#"This is \ escaped"#),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn named_dynamic_mod() {
        let input = "{color: $relevant_color | Not only content can be dynamic, also value of other metadata}";
        let expected = sections![{
            (fn Color) Dynamic::new: s("relevant_color"),
            Content: s("Not only content can be dynamic, also value of other metadata"),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn implicit_dynamic_mod() {
        let input = "{color: $ |If the identifier of a dynamic metadata value is elided, \
                then the typeid of the rust type is used}";
        let expected = sections![{
            // TODO(feat): this needs to be TypeId.of(Color)
            (fn Color) Dynamic::new: s("implicit"),
            Content: s(
                "If the identifier of a dynamic metadata value is elided, \
                then the typeid of the rust type is used"
            ),
        }];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn implicit_dynamic_content_mod() {
        let input = "can also use a single elided content if you want: {content:$}";
        let expected = sections![
            "can also use a single elided content if you want: ",
            // TODO(feat): this needs to be TypeId.of(Content)
            {(fn Content) Dynamic::new: s("implicit")},
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    #[test]
    fn all_dynamic_content_declarations() {
        let input =
            "{content:$ident} is equivalent to {ident} also {color:white| {ident}} and {  ident  } and {color:white|{ident}}.";
        let expected = sections![
            {(fn Content) Dynamic::new: s("ident")},
            " is equivalent to ",
            {(fn Content) Dynamic::new: s("ident")},
            " also ",
            {Color: Col::WHITE, (fn Content) Dynamic::new: s("ident")},
            " and ",
            {(fn Content) Dynamic::new: s("ident")},
            " and ",
            {Color: Col::WHITE, (fn Content) Dynamic::new: s("ident")},
            ".",
        ];
        assert_eq_sorted!(Ok(expected), parse(input));
    }
    // ---------------------------------
}
