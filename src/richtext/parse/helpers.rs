use std::any::TypeId;
use std::num::ParseFloatError;

use thiserror::Error;

use super::super::{color, modifiers, Content, Dynamic, Modifiers, ModifyBox, Section};

#[derive(Error, Debug)]
pub enum Error<'a> {
    #[error("{0}")]
    ColorParse(#[from] color::Error),
    #[error("{0}")]
    FloatParse(#[from] ParseFloatError),
    #[error("Tried to use an unregistered modifier: {0}")]
    UnknownModifier(&'a str),
}

pub(super) type Result<'a, T> = std::result::Result<T, Error<'a>>;

pub(super) enum ModifierValue<'a> {
    Dynamic(&'a str),
    Static(&'a str),
}
pub(super) enum Element<'a> {
    Modifier((&'a str, ModifierValue<'a>)),
    Content(Vec<Section>),
}

pub(super) fn flat_vec<T>(vs: Vec<Vec<T>>) -> Vec<T> {
    vs.into_iter().flatten().collect()
}
pub(super) fn open_section(input: &str) -> Vec<Section> {
    let content_id = TypeId::of::<Content>();

    let mut modifiers = Modifiers::new();
    modifiers.insert(content_id, Box::new(Content(input.to_owned())));

    vec![Section { modifiers }]
}
pub(super) fn short_dynamic(input: Option<&str>) -> Vec<Section> {
    // TODO: use typeid as Dynamic::new arg if None
    let content_id = TypeId::of::<Content>();
    let content_value = input.map_or_else(|| "content".to_owned(), |v| v.to_owned());

    let mut modifiers = Modifiers::new();
    modifiers.insert(content_id, Box::new(Dynamic::new(content_value)));

    vec![Section { modifiers }]
}
pub(super) fn aggregate_elements(elements: Vec<Element>) -> Result<Vec<Section>> {
    use modifiers::{Color, Font, RelSize};

    let static_modifier = |key, value: &str| -> Result<ModifyBox> {
        match key {
            "font" => Ok(Box::new(Font(value.to_owned()))),
            "color" => Ok(Box::new(value.parse::<Color>()?)),
            "size" => Ok(Box::new(RelSize(value.parse()?))),
            "content" => Ok(Box::new(Content(value.to_owned()))),
            key => Err(Error::UnknownModifier(key)),
        }
    };
    let modifier_value = |key, value| -> Result<ModifyBox> {
        match value {
            ModifierValue::Dynamic(name) => Ok(Box::new(Dynamic::new(name.to_owned()))),
            ModifierValue::Static(value) => static_modifier(key, value),
        }
    };
    let modifier_key = |key| match key {
        "font" => Ok(TypeId::of::<Font>()),
        "content" => Ok(TypeId::of::<Content>()),
        "size" => Ok(TypeId::of::<RelSize>()),
        "color" => Ok(TypeId::of::<Color>()),
        key => Err(Error::UnknownModifier(key)),
    };

    let mut modifiers = Modifiers::new();
    let mut content = open_section("");
    for element in elements.into_iter() {
        match element {
            Element::Modifier((key, value)) => {
                modifiers.insert(modifier_key(key)?, modifier_value(key, value)?);
            }
            Element::Content(sections) => content = sections,
        }
    }
    for content in &mut content {
        let clone_pair = |(x, y): (&TypeId, &ModifyBox)| (*x, y.clone());
        content.modifiers.extend(modifiers.iter().map(clone_pair));
    }
    Ok(content)
}
