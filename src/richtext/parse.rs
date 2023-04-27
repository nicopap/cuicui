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
// How to read the following code: Look at the variable names for match with the grammar,
// they are defined in the same order.
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
