//! Parse colors.

use std::{
    num::{ParseFloatError, ParseIntError},
    str::FromStr,
};

use bevy::{prelude::Color, render::color::HexColorError};
use thiserror::Error;

type Result<T> = std::result::Result<T, Error>;

// TODO: better error messages
#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "Couldn't turn this into a color: {0}\n\
        supported color formats are:\n\
        - `rgb(f32, f32, f32 [, f32]?)`: `rgb` can also be `rgb_lin` or `hsl`, \
          also works with `u8`s instead of `f32`s.\n\
        - `color_name`: as in bevy [`Color`] constant names.\n\
        - `#EC5EC5`: html-style hex value (requires # prefix)\n"
    )]
    BadColor(String),
    #[error(
        "Attempted to use `{name}` to define a color, it expects 1, 3 or 4 \
        arguments, but {count} were given."
    )]
    BadArgCount { name: &'static str, count: u32 },
    #[error("Failed to convert number in parameter color definition: {0}")]
    BadF32(#[from] ParseFloatError),
    #[error("Failed to convert number in parameter color definition: {0}")]
    BadU8(#[from] ParseIntError),
    #[error("{0}")]
    BadHex(#[from] HexColorError),
    #[error("color definition using {prefix} is missing a closing `)`")]
    UnclosedArg { prefix: &'static str },
}

fn parameter(color: &str) -> Option<Result<Color>> {
    let num = |text: &str| -> Result<f32> {
        let text = text.trim();
        if text.contains('.') {
            Ok(text.parse()?)
        } else {
            let u8 = u8::from_str(text)?;
            Ok((u8 as f32) / (u8::MAX as f32))
        }
    };
    let (prefix, get_color, color): (&'static str, fn(f32, f32, f32, f32) -> Color, &str) =
        if let Some(color) = color.strip_prefix("rgb(") {
            ("rgb(", Color::rgba, color)
        } else if let Some(color) = color.strip_prefix("rgb_lin(") {
            ("rgb_lin(", Color::rgba_linear, color)
        } else if let Some(color) = color.strip_prefix("hsl(") {
            ("hsl(", Color::hsla, color)
        } else {
            return None;
        };
    let run = || {
        let len = color.len() - 1;
        if Some(")") != color.get(len..) {
            return Err(Error::UnclosedArg { prefix });
        }
        let args = &color[..len];
        let args = args.split(',').collect::<Vec<_>>();
        let (r, g, b, a) = match *args.as_slice() {
            [r, g, b, a] => (num(r)?, num(g)?, num(b)?, num(a)?),
            [r, g, b] => (num(r)?, num(g)?, num(b)?, 1.0),
            [r] => (num(r)?, num(r)?, num(r)?, 1.0),
            ref v => return Err(Error::BadArgCount { name: prefix, count: v.len() as u32 }),
        };
        Ok(get_color(r, g, b, a))
    };
    Some(run())
}
fn hex(color: &str) -> Option<Result<Color>> {
    if !color.starts_with('#') {
        return None;
    }
    Some(Color::hex(&color[1..]).map_err(From::from))
}
fn named(color: &str) -> Option<Color> {
    match color {
        "alice_blue" => Some(Color::ALICE_BLUE),
        "antique_white" => Some(Color::ANTIQUE_WHITE),
        "aquamarine" => Some(Color::AQUAMARINE),
        "azure" => Some(Color::AZURE),
        "beige" => Some(Color::BEIGE),
        "bisque" => Some(Color::BISQUE),
        "black" => Some(Color::BLACK),
        "blue" => Some(Color::BLUE),
        "crimson" => Some(Color::CRIMSON),
        "cyan" => Some(Color::CYAN),
        "dark_gray" => Some(Color::DARK_GRAY),
        "dark_green" => Some(Color::DARK_GREEN),
        "fuchsia" => Some(Color::FUCHSIA),
        "gold" => Some(Color::GOLD),
        "gray" => Some(Color::GRAY),
        "green" => Some(Color::GREEN),
        "indigo" => Some(Color::INDIGO),
        "lime_green" => Some(Color::LIME_GREEN),
        "maroon" => Some(Color::MAROON),
        "midnight_blue" => Some(Color::MIDNIGHT_BLUE),
        "navy" => Some(Color::NAVY),
        "none" => Some(Color::NONE),
        "olive" => Some(Color::OLIVE),
        "orange" => Some(Color::ORANGE),
        "orange_red" => Some(Color::ORANGE_RED),
        "pink" => Some(Color::PINK),
        "purple" => Some(Color::PURPLE),
        "red" => Some(Color::RED),
        "salmon" => Some(Color::SALMON),
        "sea_green" => Some(Color::SEA_GREEN),
        "silver" => Some(Color::SILVER),
        "teal" => Some(Color::TEAL),
        "tomato" => Some(Color::TOMATO),
        "turquoise" => Some(Color::TURQUOISE),
        "violet" => Some(Color::VIOLET),
        "white" => Some(Color::WHITE),
        "yellow" => Some(Color::YELLOW),
        "yellow_green" => Some(Color::YELLOW_GREEN),
        _ => None,
    }
}
pub(super) fn parse(input: &str) -> Result<Color> {
    let err = || Error::BadColor(input.to_owned());
    let color = input.trim().to_lowercase();
    named(&color)
        .map(Ok)
        .or_else(|| hex(&color))
        .or_else(|| parameter(&color))
        .ok_or_else(err)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_named() {
        let color = parse("violet").unwrap();
        assert_eq!(color, Color::VIOLET);

        let color = parse("Yellow").unwrap();
        assert_eq!(color, Color::YELLOW);

        let color = parse("RED").unwrap();
        assert_eq!(color, Color::RED);
    }
    #[test]
    fn invalid_named() {
        assert!(parse("deep_purple").is_err());
        assert!(parse("klein_blue").is_err());
    }
    #[test]
    fn valid_parameters() {
        let color = parse("rgb(10, 20, 30)").unwrap();
        assert_eq!(color, Color::rgba_u8(10, 20, 30, 255));

        let color = parse("rgb(1.0, 0.1, 0.5)").unwrap();
        assert_eq!(color, Color::rgba(1.0, 0.1, 0.5, 1.0));

        let color = parse("rgb_lin(1.0, 0.1, 0.5, 1.0)").unwrap();
        assert_eq!(color, Color::rgba_linear(1.0, 0.1, 0.5, 1.0));

        let color = parse("rgb(10,34,   102)").unwrap();
        assert_eq!(color, Color::rgba_u8(10, 34, 102, 255));

        let color = parse("hsl(330.0, 0.5,0.5)").unwrap();
        assert_eq!(color, Color::hsla(330.0, 0.5, 0.5, 1.0));

        let color = parse("hsl(3.141516, 0.1,0.99   ,1.0)").unwrap();
        assert_eq!(color, Color::hsla(3.141516, 0.1, 0.99, 1.0));

        let color = parse("rgb(        233       )").unwrap();
        assert_eq!(color, Color::rgba_u8(233, 233, 233, 255));
    }
    #[test]
    fn invalid_parameters() {
        assert!(parse("rgb(1000, 3434, 2223)").is_err());
        assert!(parse("rgb_lin(1.0, 0.1, 0.5, 1.0, 1.0, 1.0)").is_err());
        assert!(parse("rgb_lin(1.0, 1.0)").is_err());
        assert!(parse("rgb_lin(10, 34)").is_err());
        assert!(parse("hsl(330.0, 0.5,0.5").is_err());
        // FIXME
        // assert!(parse("hsl(10,34    , 102)").is_err());
    }
    #[test]
    fn valid_hex() {
        let color = parse("#343434").unwrap();
        assert_eq!(color, Color::hex("343434").unwrap());

        let color = parse("#baddab").unwrap();
        assert_eq!(color, Color::hex("baddab").unwrap());

        let color = parse("#F00B4D").unwrap();
        assert_eq!(color, Color::hex("F00B4D").unwrap());
    }
    #[test]
    fn invalid_hex() {
        assert!(parse("#fr3ncH").is_err());
        assert!(parse("#1").is_err());
        assert!(parse("#1234567890").is_err());
    }
}
