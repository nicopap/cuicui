//! Parse rich text according to spec
use thiserror::Error;

use super::{Color, Font, RelSize, RichText, Section};

// FIXME: Write lot of tests

#[derive(Copy, Clone, PartialEq)]
enum Modifier {
    RelSize,
    Font,
    Color,
}
#[derive(Copy, Clone, PartialEq)]
enum ModifierType {
    Dynamic,
    Static,
}

#[derive(Error, Debug)]
pub enum Error {}

type Result<T> = std::result::Result<T, Error>;

fn value(input: &mut &str) -> Result<(ModifierType, &str)> {}
fn style_type(input: &mut &str) -> Result<StyleType> {}
fn element(input: &mut &str) -> Result<(StyleType, &str)> {}
fn section(input: &mut &str) -> Result<Option<Section>> {
    if input.len() == 0 {
        return Ok(None);
    }
}
pub(super) fn rich_text(mut input: &str) -> Result<Vec<Section>> {
    let mut sections = Vec::new();
    loop {
        match section(&mut input)? {
            Some(section) => sections.push(section),
            None => return Ok(sections),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value() {
        todo!();
    }
    #[test]
    fn style_type() {
        todo!();
    }
    #[test]
    fn element() {
        todo!();
    }
    #[test]
    fn section() {
        todo!();
    }
    #[test]
    fn rich_text() {
        todo!();
    }
}
