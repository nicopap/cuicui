use std::{any::TypeId, borrow::Cow, mem, num::ParseFloatError};

use thiserror::Error;
use winnow::{
    character::multispace0, error::ParseError, sequence::delimited, stream::Accumulate,
    stream::AsChar, stream::Stream, stream::StreamIsPartial, Parser,
};

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

#[derive(Debug)]
pub(super) struct Sections(pub(super) Vec<Section>);
impl Sections {
    pub(super) fn tail((head, mut tail): (Option<Section>, Self)) -> Self {
        if let Some(head) = head {
            tail.0.insert(0, head);
        }
        tail
    }
}
impl Accumulate<Vec<Section>> for Sections {
    fn initial(capacity: Option<usize>) -> Self {
        Self(Vec::with_capacity(capacity.unwrap_or(0)))
    }
    fn accumulate(&mut self, acc: Vec<Section>) {
        self.0.extend(acc)
    }
}
impl Accumulate<(Vec<Section>, Option<Section>)> for Sections {
    fn initial(capacity: Option<usize>) -> Self {
        Self(Vec::with_capacity(capacity.unwrap_or(0) * 2))
    }
    fn accumulate(&mut self, (closed, opt_open): (Vec<Section>, Option<Section>)) {
        self.0.extend(closed);
        self.0.extend(opt_open);
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
fn escape_backslash(input: &mut String) {
    let last_was_esc = &mut false;
    input.retain(|c| {
        let is_backslash = c == '\\';
        mem::replace(last_was_esc, is_backslash) || !is_backslash
    });
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
    fn escape_values(&mut self) {
        let Self::Static(value) = self else { return; };
        let value = value.to_mut();
        escape_backslash(value);
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
impl Section {
    pub(super) fn opt_from(input: &str) -> Option<Self> {
        if input.is_empty() {
            return None;
        }
        let content_id = TypeId::of::<Content>();

        let mut modifiers = Modifiers::new();
        let mut escaped = input.to_owned();
        escape_backslash(&mut escaped);
        modifiers.insert(content_id, Box::new(Content(escaped)));

        Some(Section { modifiers })
    }
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
    (elements, content): (Vec<Element>, Option<Sections>),
) -> Result<Vec<Section>> {
    use modifiers::{Color, Font, RelSize};

    // TODO(correct): check if empty Content (should never happen)

    let static_modifier = |key, value: Cow<str>| -> Result<ModifyBox> {
        match key {
            "font" => Ok(Box::new(Font(value.into()))),
            "color" => Ok(Box::new(value.parse::<Color>()?)),
            "size" => Ok(Box::new(RelSize(value.parse()?))),
            "content" => Ok(Box::new(Content(value.into()))),
            key => Err(Error::UnknownModifier(key)),
        }
    };
    let modifier_value = |key, mut value: ModifierValue| -> Result<ModifyBox> {
        value.escape_values();
        match value {
            ModifierValue::Dynamic(name) => Ok(Box::new(Dynamic::new(name.to_string()))),
            ModifierValue::Static(value) => static_modifier(key, value),
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
    // TODO(clean): This might be error prone, why do we initialize `sections`
    // first, then add content? Does the default `section` mean anything?
    let mut sections = vec![Section::default()];
    for Element { key, value } in elements.into_iter() {
        modifiers.insert(modifier_key(key)?, modifier_value(key, value)?);
    }
    if modifiers.contains_key(&TypeId::of::<Content>()) && content.is_some() {
        return Err(Error::TwoContents);
    } else if let Some(Sections(content)) = content {
        sections = content;
    }
    for section in &mut sections {
        let clone_pair = |(x, y): (&TypeId, &ModifyBox)| (*x, y.clone());
        section.modifiers.extend(modifiers.iter().map(clone_pair));
    }
    Ok(sections)
}

pub(super) fn ws<I, O, E>(inner: impl Parser<I, O, E>) -> impl Parser<I, O, E>
where
    <I as Stream>::Token: AsChar,
    <I as Stream>::Token: Copy,
    I: StreamIsPartial + Stream,
    E: ParseError<I>,
{
    delimited(multispace0, inner, multispace0)
}
