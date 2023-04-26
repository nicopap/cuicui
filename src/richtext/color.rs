use std::str::FromStr;

use bevy::prelude::Color;
use thiserror::Error;

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
}

fn parameter(color: &str) -> Option<Color> {
    let num = |text: &str| {
        let text = text.trim();
        if text.contains('.') {
            f32::from_str(text).ok()
        } else {
            let u8 = u8::from_str(text).ok()?;
            Some((u8 as f32) / (u8::MAX as f32))
        }
    };
    let check_prefix = |prefix: &'static str, get_color: fn(f32, f32, f32, f32) -> Color| {
        let unprefixed = color.strip_prefix(prefix)?;
        let len = unprefixed.len() - 1;
        let args = unprefixed.get(..len)?;
        let args = args.split(',').take(4).collect::<Vec<_>>();
        let (r, g, b, a) = match *args.as_slice() {
            [r, g, b, a] => (num(r)?, num(g)?, num(b)?, num(a)?),
            [r, g, b] => (num(r)?, num(g)?, num(b)?, 1.0),
            [r] => (num(r)?, num(r)?, num(r)?, 1.0),
            // TODO: return a meaningfull error
            _ => return None,
        };
        Some(get_color(r, g, b, a))
    };
    check_prefix("rgb(", Color::rgba)
        .or_else(|| check_prefix("rgb_lin(", Color::rgba_linear))
        .or_else(|| check_prefix("hsl(", Color::hsla))
}
fn hex(color: &str) -> Option<Color> {
    if !color.starts_with('#') {
        return None;
    }
    Color::hex(&color[1..]).ok()
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

impl FromStr for super::Color {
    type Err = Error;

    fn from_str(color: &str) -> Result<Self, Self::Err> {
        let err = || Error::BadColor(color.to_owned());
        let color = color.to_lowercase();
        let color = named(&color)
            .or_else(|| hex(&color))
            .or_else(|| parameter(&color))
            .ok_or_else(err)?;
        Ok(super::Color(color))
    }
}
