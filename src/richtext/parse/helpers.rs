use std::num::ParseFloatError;
use std::{any::TypeId, borrow::Cow};

use thiserror::Error;
use winnow::stream::Accumulate;

use super::super::{color, modifiers, Content, Dynamic, Modifiers, ModifyBox, RichText, Section};

#[derive(Error, Debug)]
pub enum Error<'a> {
    #[error("{0}")]
    ColorParse(#[from] color::Error),
    #[error("{0}")]
    FloatParse(#[from] ParseFloatError),
    #[error("Tried to use an unregistered modifier: {0}")]
    UnknownModifier(&'a str),
    #[error(
        "Both a trailing content section and a modifier declaration \
        content exist in section, those are mutually exclusive"
    )]
    TwoContents,
}

pub(super) type Result<'a, T> = std::result::Result<T, Error<'a>>;

pub(super) struct Sections(Vec<Section>);
impl Accumulate<Vec<Section>> for Sections {
    fn initial(capacity: Option<usize>) -> Self {
        Self(Vec::with_capacity(capacity.unwrap_or(0)))
    }

    fn accumulate(&mut self, acc: Vec<Section>) {
        self.0.extend(acc)
    }
}
impl From<Sections> for RichText {
    fn from(value: Sections) -> Self {
        RichText { sections: value.0 }
    }
}

#[derive(Clone, Debug)]
pub(super) enum ModifierValue<'a> {
    Dynamic(Cow<'a, str>),
    Static(Cow<'a, str>),
    DynamicImplicit,
}
impl<'a> ModifierValue<'a> {
    pub(super) fn dyn_opt(input: Option<&'a str>) -> Self {
        match input {
            Some(dynamic) => Self::Dynamic(dynamic.into()),
            None => Self::DynamicImplicit,
        }
    }
    pub(super) fn statik(input: &'a str) -> Self {
        Self::Static(input.into())
    }
}
#[derive(Debug)]
pub(super) struct Element<'a> {
    pub(super) key: &'a str,
    pub(super) value: ModifierValue<'a>,
}
impl<'a> Element<'a> {
    pub(super) fn modifier((key, value): (&'a str, ModifierValue<'a>)) -> Self {
        Element { key, value }
    }
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
pub(super) fn elements_and_content(
    (elements, content): (Vec<Element>, Option<Vec<Section>>),
) -> Result<Vec<Section>> {
    use modifiers::{Color, Font, RelSize};

    // TODO(correct): check if empty Content (should never happen)

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
            ModifierValue::Dynamic(name) => Ok(Box::new(Dynamic::new(name.to_string()))),
            ModifierValue::Static(value) => static_modifier(key, value.as_ref()),
            ModifierValue::DynamicImplicit => Ok(Box::new(Dynamic::new("implicit".to_owned()))),
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
    let mut sections = open_section("");
    for Element { key, value } in elements.into_iter() {
        modifiers.insert(modifier_key(key)?, modifier_value(key, value)?);
    }
    for section in &mut sections {
        let clone_pair = |(x, y): (&TypeId, &ModifyBox)| (*x, y.clone());
        section.modifiers.extend(modifiers.iter().map(clone_pair));
    }
    if modifiers.contains_key(&TypeId::of::<Content>()) && content.is_some() {
        return Err(Error::TwoContents);
    } else if let Some(content) = content {
        sections = content;
    }
    Ok(sections)
}
