//! Parse rich text according to spec
mod helpers;

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{alpha1, alphanumeric1, multispace0},
    combinator::{map, map_res, opt, recognize},
    multi::{many0, many0_count, many1},
    sequence::{delimited, pair, preceded, separated_pair, terminated},
    Finish, IResult,
};
use thiserror::Error;

use super::{RichText, Section};
use helpers::{aggregate_elements, flat_vec, open_section, short_dynamic, Element, ModifierValue};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Nom(#[from] nom::error::Error<String>),
    #[error("Not all of the input was correctly parsed, remaining was: \"{0}\"")]
    Trailing(String),
}

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
fn ident(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0_count(alt((alphanumeric1, tag("_")))),
    ))(input)
}
fn closed(input: &str) -> IResult<&str, Vec<Section>> {
    let colon = terminated(tag(":"), multispace0);
    let bar = delimited(multispace0, tag("|"), multispace0);
    let open_b = terminated(tag("{"), multispace0);
    let close_b = preceded(multispace0, tag("}"));

    let text = is_not;
    let balanced_text = is_not;

    // TODO: dynamic tags
    let key = alt((tag("font"), tag("content"), tag("size"), tag("color")));

    let section_text = map(text("{}"), open_section);
    let sub_section = alt((delimited(open_b, closed, close_b), section_text));

    // TODO: performance
    let bare_content = map(many1(sub_section), flat_vec);

    let metadata = alt((
        map(preceded(tag("$"), ident), ModifierValue::Dynamic),
        map(balanced_text(",}"), ModifierValue::Static),
    ));
    let closed_element = alt((
        map(separated_pair(key, colon, metadata), Element::Modifier),
        map(preceded(bar, bare_content), Element::Content),
    ));
    let all_elements = map_res(many1(closed_element), aggregate_elements);

    let mut closed = alt((map(opt(ident), short_dynamic), all_elements));

    closed(input)
}
pub(super) fn rich_text(input: &str) -> Result<RichText, Error> {
    let open_b = terminated(tag("{"), multispace0);
    let close_b = preceded(multispace0, tag("}"));

    let text = is_not;

    let section_text = map(text("{"), open_section);
    let section = alt((delimited(open_b, closed, close_b), section_text));

    let mut rich_text = many0(section);

    let result = rich_text(input);
    let (remaining, sections) = result.map_err(|e| e.to_owned()).finish()?;
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

    #[test]
    fn valid_richtext() {
        let s = |input: &'static str| String::from(input);
        let valid = &[
            (
                "This is some text, it is just a single content section",
                sections!["This is some text, it is just a single content section"],
            ),
            (
                "This one contains a single {dynamic_content} that can be replaced at runtime",
                sections![
                    "This one contains a single ",
                    {(fn Content) Dynamic::new : s("dynamic_content")},
                    " that can be replaced at runtime",
                ],
            ),
            (
                "{|This is also just some non-dynamic text, commas need not be escaped}",
                sections!["This is also just some non-dynamic text, commas need not be escaped"],
            ),
            (
                r#"{content: This may also work\, but commas need to be escaped}"#,
                sections!["This may also work, but commas need to be escaped"],
            ),
            (
                "{dynamic_content}",
                sections![{(fn Content) Dynamic::new : s("dynamic_content")}],
            ),
            (
                "{}",
                // TODO: this needs to be TypeId.of(Content)
                sections![{(fn Content) Dynamic::new : s("content")}],
            ),
            (
                "An empty {} is equivalent to {name}, but referred by typeid instead of name",
                sections![
                    "An empty ",
                    // TODO: this needs to be TypeId.of(Content)
                    {(fn Content) Dynamic::new : s("content")},
                    " is equivalent to ",
                    {(fn Content) Dynamic::new : s("name")},
                    ", but referred by typeid instead of name"
                ],
            ),
            (
                "{color: Blue | This text is blue}",
                sections![
                    {Color: Col::BLUE, Content: s("This text is blue")},
                ],
            ),
            (
                "{color: Blue | {dynamic_blue_content}}",
                sections![{
                    Color: Col::BLUE,
                    (fn Content) Dynamic::new: s("dynamic_blue_content"),
                }],
            ),
            (
                "{color: Blue | This is non-bold text: {font:b|now it is bold, \
                you may also use {size:1.3|{deeply_nested}} sections}, not anymore {font:i|yet again}!}",
                sections![
                    { Color: Col::BLUE, Content: s("This is non-bold text: ") },
                    { Color: Col::BLUE, Font: s("b"), Content: s("now it is bold, you may also use ") },
                    { Color: Col::BLUE, Font: s("b"), RelSize: 1.3, (fn Content) Dynamic::new: s("deeply_nested") },
                    { Color: Col::BLUE, Font: s("b"), Content: s(" sections") },
                    { Color: Col::BLUE, Content: s(", not anymore ") },
                    { Color: Col::BLUE, Font: s("i"), Content: s("yet again") },
                ],
            ),
            (
                "{color:Red| Some red text}, some default color {dynamic_name}. \
                {color:pink|And pink, why not?}",
                sections![
                    { Color: Col::RED, Content: s("Some red text") },
                    ", some default color ",
                    { (fn Content) Dynamic::new: s("dynamic_name") },
                    ". ",
                    { Color: Col::PINK, Content: s("And pink, why not?") },
                ],
            ),
            (
                "{color:rgb(12, 34, 50),font:bold.ttf|metadata values} can contain \
                commas within parenthesis or square brackets",
                sections![
                    { Color: Col::rgb_u8(12,34,50), Font: s("bold.ttf"), Content: s("metadata values") },
                    " can contain commas within parenthesis or square brackets",
                ],
            ),
            (
                r#"You can escape \{ curly brackets \}."#,
                sections!["You can escape { curly brackets }."],
            ),
            (
                r#"{color: pink| even inside \{ a closed section \}}."#,
                sections![{
                    Color: Col::PINK,
                    Content: s("even inside { a closed section }"),
                }],
            ),
            (
                "{color: $relevant_color | Not only content can be dynamic, also value of other metadata}",
                sections![{
                    (fn Color) Dynamic::new: s("relevant_color"),
                    Content: s("Not only content can be dynamic, also value of other metadata"),
                }],
            ),
            (
                "{color: $ |If the identifier of a dynamic metadata value is elided, \
                then the typeid of the rust type is used}",
                sections![{
                    // TODO: this needs to be TypeId.of(Color)
                    (fn Color) Dynamic::new: s("color"),
                    Content: s(
                        "If the identifier of a dynamic metadata value is elided, \
                        then the typeid of the rust type is used"
                    ),
                }],
            ),
            (
                "can also use a single elided content if you want: {content:$}",
                sections![
                    "can also use a single elided content if you want: ",
                    // TODO: this needs to be TypeId.of(Content)
                    {(fn Content) Dynamic::new: s("content")},
                ],
            ),
            (
                "{content:$ident} is equivalent to {ident} also {| {ident} } and \
                {  ident  } and {|{ident}}.",
                sections![
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
                ],
            ),
        ];
        for (string, sections) in valid {
            todo!()
        }
    }
}
