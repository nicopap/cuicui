//! Parse rich text according to spec
mod helpers;

use nom::{
    branch::alt,
    bytes::complete::{escaped, is_not, tag},
    character::complete::{alpha1, alphanumeric1, multispace0, one_of},
    combinator::{map, map_res, opt, recognize, value},
    error::VerboseError,
    multi::{many0, many0_count, separated_list0},
    sequence::{delimited, pair, preceded, separated_pair, terminated},
    Finish,
};
use thiserror::Error;

use super::{RichText, Section};
use helpers::{aggregate_elements, flat_vec, open_section, short_dynamic, Element, ModifierValue};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Nom(#[from] VerboseError<String>),
    #[error(transparent)]
    NomBad(#[from] nom::error::Error<String>),
    #[error("Not all of the input was correctly parsed, remaining was: \"{0}\"")]
    Trailing(String),
}
impl From<VerboseError<&'_ str>> for Error {
    fn from(value: VerboseError<&'_ str>) -> Self {
        let errors = value
            .errors
            .into_iter()
            .map(|(m, k)| (m.to_owned(), k))
            .collect();
        Self::Nom(VerboseError { errors })
    }
}

type IResult<'a, I, O> = nom::IResult<I, O, VerboseError<&'a str>>;

// ```
// key = 'font' | 'content' | 'size' | 'color'
// sub_section = '{' closed '}' | <text∉{}>
// bare_content = (sub_section)+
// metadata = '$' <ident> | <balanced_text∉,}>
// closed_element = key ':' metadata | '|' bare_content
// closed = '' | <ident> | (closed_element)+
// section = '{' closed '}' | <text∉{>
// rich_text = (section)*
// ```
//
// How to read the following code:
// Look at the variable names for match with the grammar, they are defined in the same order.
// fn text(exclude: &str, input: &str) -> IResult<&str, &str> {}
fn scope(input: &str) -> IResult<&str, &str> {
    let escaped = || escaped(is_not("()[]{}\\"), '\\', one_of("()[]{},|\\"));

    let scope = || separated_list0(scope, opt(escaped()));
    recognize(alt((
        delimited(tag("{"), scope(), tag("}")),
        delimited(tag("["), scope(), tag("]")),
        delimited(tag("("), scope(), tag(")")),
    )))(input)
}
fn balanced_text(input: &str) -> IResult<&str, &str> {
    let escape = |inner| escaped(inner, '\\', one_of("()[]{},|\\"));

    let outer = escape(is_not("([{,|\\"));

    recognize(separated_list0(scope, opt(outer)))(input)
}
fn ident(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0_count(alt((alphanumeric1, tag("_")))),
    ))(input)
}
fn closed(input: &str) -> IResult<&str, Vec<Section>> {
    use ModifierValue as Mod;

    let colon = terminated(tag(":"), multispace0);
    let bar = delimited(multispace0, tag("|"), multispace0);
    let open_b = terminated(tag("{"), multispace0);
    let close_b = preceded(multispace0, tag("}"));

    let text = |exclude| escaped(is_not(exclude), '\\', one_of("{},\\"));

    // TODO: dynamic tags
    let key = alt((tag("font"), tag("content"), tag("size"), tag("color")));

    let section_text = map(text("{}\\"), open_section);
    let sub_section = alt((delimited(open_b, closed, close_b), section_text));

    // TODO: performance
    let bare_content = map(many0(sub_section), flat_vec);

    let metadata = alt((
        map(preceded(tag("$"), ident), |t| Mod::Dynamic(t.into())),
        value(Mod::DynamicImplicit, tag("$")),
        map(balanced_text, |t: &str| Mod::Static(t.into())),
    ));
    let closed_element = alt((
        map(separated_pair(key, colon, metadata), Element::Modifier),
        map(preceded(bar, bare_content), Element::Content),
    ));
    let all_elements = map_res(many0(closed_element), aggregate_elements);

    let mut closed = alt((map(opt(ident), short_dynamic), all_elements));

    closed(input)
}
pub(super) fn rich_text(input: &str) -> Result<RichText, Error> {
    let open_b = terminated(tag("{"), multispace0);
    let close_b = preceded(multispace0, tag("}"));

    let text = |exclude| escaped(is_not(exclude), '\\', one_of("{},\\"));

    let section_text = map(text("{\\"), open_section);
    let section = alt((delimited(open_b, closed, close_b), section_text));

    let mut rich_text = many0(section);

    let (remaining, sections) = rich_text(input).finish()?;
    if remaining.is_empty() {
        Ok(RichText { sections: flat_vec(sections) })
    } else {
        Err(Error::Trailing(remaining.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use bevy::prelude::Color as Col;
    use pretty_assertions_sorted::assert_eq_sorted;

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
            sections!(@modifiers Content($text.to_owned()))
        };
        (@item {$( $(( fn $type_id:ident ))? $($modifier:ident )::* : $value:expr ),* $(,)? }) => {
            sections!(@modifiers $( $((fn $type_id))? $($modifier)::*($value) ),*)
        };
        ($( $item:tt ),* $(,)?) => {
            vec![ $( sections!(@item $item) ),* ]
        }
    }
    fn s(input: &str) -> String {
        String::from(input)
    }

    #[test]
    fn balanced_text_valid() {
        let parse = |input| balanced_text(input).finish().unwrap().0;
        let complete = [
            "",
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
            "foo bar",
            r#"foo \| bar"#,
            "foo [] bar",
            "foo ({},[]) bar",
            r#"foo \, bar"#,
            "foo (|) bar",
            r#"(foo \{ bar)"#,
            r#"(foo \{ |)"#,
        ];
        for input in &complete {
            assert_eq_sorted!(parse(input), "");
        }
    }
    #[test]
    fn balanced_text_invalid() {
        let parse = |input| balanced_text(input).finish().unwrap().0;
        let incomplete = [
            ("foo , bar", ", bar"),
            (",", ","),
            // ("(", ""),
            // ("foo ( bar", ""),
            ("foo | bar", "| bar"),
            (r#"foo \, , bar"#, ", bar"),
            (r#"foo , \, bar"#, r#", \, bar"#),
            (r#"foo \( , \) bar"#, r#", \) bar"#),
        ];
        for (input, remaining) in &incomplete {
            assert_eq_sorted!(parse(input), *remaining);
        }
    }

    // ---------------------------------
    //        test rich_text parsing
    // ---------------------------------
    fn parse(input: &str) -> Result<Vec<Section>, String> {
        rich_text(input)
            .map(|t| t.sections)
            .map_err(|s| s.to_string())
    }
    #[test]
    fn plain_text() {
        let input = "This is some text, it is just a single content section";
        let expected = sections!["This is some text, it is just a single content section"];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn single_dynamic_shorthand() {
        let input = "This one contains a single {dynamic_content} that can be replaced at runtime";
        let expected = sections![
            "This one contains a single ",
            {(fn Content) Dynamic::new : s("dynamic_content")},
            " that can be replaced at runtime",
        ];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn closed_content() {
        let input = "{|This is also just some non-dynamic text, commas need not be escaped}";
        let expected =
            sections!["This is also just some non-dynamic text, commas need not be escaped"];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn closed_explicit_content_escape_comma() {
        let input = r#"{content: This may also work\, but commas need to be escaped}"#;
        let expected = sections!["This may also work, but commas need to be escaped"];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn outer_dynamic_shorthand() {
        let input = "{dynamic_content}";
        let expected = sections![{(fn Content) Dynamic::new : s("dynamic_content")}];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn outer_dynamic_content_implicit() {
        let input = "{}";
        let expected=
                // TODO: this needs to be TypeId.of(Content)
                sections![{(fn Content) Dynamic::new : s("content")}];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn dynamic_content_implicit() {
        let input = "An empty {} is equivalent to {name}, but referred by typeid instead of name";
        let expected = sections![
            "An empty ",
            // TODO: this needs to be TypeId.of(Content)
            {(fn Content) Dynamic::new : s("content")},
            " is equivalent to ",
            {(fn Content) Dynamic::new : s("name")},
            ", but referred by typeid instead of name"
        ];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn outer_color_mod() {
        let input = "{color: Blue | This text is blue}";
        let expected = sections![
            {Color: Col::BLUE, Content: s("This text is blue")},
        ];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn nested_dynamic_shorthand() {
        let input = "{color: Blue | {dynamic_blue_content}}";
        let expected = sections![{
            Color: Col::BLUE,
            (fn Content) Dynamic::new: s("dynamic_blue_content"),
        }];
        assert_eq_sorted!(parse(input), Ok(expected));
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
        ];
        assert_eq_sorted!(parse(input), Ok(expected));
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
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn fancy_color_multiple_mods() {
        let input = "{color:rgb(12, 34, 50),font:bold.ttf|metadata values} can contain \
                commas within parenthesis or square brackets";
        let expected = sections![
            { Color: Col::rgb_u8(12,34,50), Font: s("bold.ttf"), Content: s("metadata values") },
            " can contain commas within parenthesis or square brackets",
        ];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn escape_curlies_outer() {
        let input = r#"You can escape \{ curly brackets \}."#;
        let expected = sections!["You can escape { curly brackets }."];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn escape_curlies_inner() {
        let input = r#"{color: pink| even inside \{ a closed section \}}."#;
        let expected = sections![{
            Color: Col::PINK,
            Content: s("even inside { a closed section }"),
        }];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn named_dynamic_mod() {
        let input = "{color: $relevant_color | Not only content can be dynamic, also value of other metadata}";
        let expected = sections![{
            (fn Color) Dynamic::new: s("relevant_color"),
            Content: s("Not only content can be dynamic, also value of other metadata"),
        }];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn implicit_dynamic_mod() {
        let input = "{color: $ |If the identifier of a dynamic metadata value is elided, \
                then the typeid of the rust type is used}";
        let expected = sections![{
            // TODO: this needs to be TypeId.of(Color)
            (fn Color) Dynamic::new: s("color"),
            Content: s(
                "If the identifier of a dynamic metadata value is elided, \
                then the typeid of the rust type is used"
            ),
        }];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn implicit_dynamic_content_mod() {
        let input = "can also use a single elided content if you want: {content:$}";
        let expected = sections![
            "can also use a single elided content if you want: ",
            // TODO: this needs to be TypeId.of(Content)
            {(fn Content) Dynamic::new: s("content")},
        ];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    #[test]
    fn all_dynamic_content_declarations() {
        let input =
            "{content:$ident} is equivalent to {ident} also {| {ident} } and {  ident  } and {|{ident}}.";
        let expected = sections![
            {(fn Content) Dynamic::new: s("ident")},
            " is equivalent to ",
            {(fn Content) Dynamic::new: s("ident")},
            " also ",
            {(fn Content) Dynamic::new: s("ident")},
            " and ",
            {(fn Content) Dynamic::new: s("ident")},
            " and ",
            {(fn Content) Dynamic::new: s("ident")},
            ".",
        ];
        assert_eq_sorted!(parse(input), Ok(expected));
    }
    // ---------------------------------
}
