//! Intermediate parsing representation.

use crate::{modifiers, show::RuntimeFormat, Modify};

use winnow::stream::Accumulate;

#[derive(Debug, PartialEq, Clone, Copy)]
pub(super) struct Modifier<'a> {
    pub(super) name: &'a str,
    pub(super) value: Dyn<'a>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(super) enum Dyn<'a> {
    Dynamic(Dynamic<'a>),
    Static(&'a str),
}

#[derive(Debug, PartialEq, Clone)]
pub(super) struct Section<'a> {
    pub(super) modifiers: Vec<Modifier<'a>>,
    pub(super) content: Dyn<'a>,
}

#[derive(Debug, PartialEq, Clone)]
pub(super) struct Full<'a>(Vec<Section<'a>>);

#[derive(Debug, PartialEq, Clone)]
pub(super) struct Sections<'a>(pub(super) Vec<Section<'a>>);

#[derive(PartialEq, Debug, Clone, Copy)]
pub(super) enum Format<'a> {
    None,
    UserDefined(&'a str),
    Fmt(RuntimeFormat),
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub(super) enum Access<'a> {
    TypeBound,
    Bound(&'a str),
    AtPath(&'a str),
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub(super) struct Dynamic<'a> {
    format: Format<'a>,
    access: Access<'a>,
}

impl<'a> Dynamic<'a> {
    pub(super) fn new((access, format): (Access<'a>, Option<Format<'a>>)) -> Self {
        Dynamic { access, format: format.unwrap_or(Format::None) }
    }
}

impl<'a> Sections<'a> {
    pub(super) fn tail((head, mut tail): (Option<Section<'a>>, Self)) -> Self {
        if let Some(head) = head {
            tail.0.insert(0, head);
        }
        tail
    }
}
impl<'a> Accumulate<Vec<Section<'a>>> for Sections<'a> {
    fn initial(capacity: Option<usize>) -> Self {
        Self(Vec::with_capacity(capacity.unwrap_or(0)))
    }
    fn accumulate(&mut self, acc: Vec<Section<'a>>) {
        self.0.extend(acc)
    }
}
impl<'a> Accumulate<(Vec<Section<'a>>, Option<Section<'a>>)> for Sections<'a> {
    fn initial(capacity: Option<usize>) -> Self {
        Self(Vec::with_capacity(capacity.unwrap_or(4) * 2))
    }
    fn accumulate(&mut self, (closed, opt_open): (Vec<Section<'a>>, Option<Section<'a>>)) {
        self.0.extend(closed);
        self.0.extend(opt_open);
    }
}

impl<'a> Modifier<'a> {
    pub(super) fn new((name, value): (&'a str, Dyn<'a>)) -> Self {
        Self { name, value }
    }
}
impl<'a> Section<'a> {
    /// A section built from plain text. If the text is empty, then there is
    /// no section.
    pub(super) fn free(input: &'a str) -> Option<Self> {
        if input.is_empty() {
            return None;
        }
        Some(Section { content: Dyn::Static(input), modifiers: Vec::new() })
    }
    /// A delimited section (ie between {}).
    pub(super) fn format(input: Dynamic) -> Vec<Self> {
        vec![Section { modifiers: vec![], content: Dyn::Dynamic(input) }]
    }
}
pub(super) fn flatten_section<'a>(
    (mut modifiers, content): (Vec<Modifier<'a>>, Option<Sections<'a>>),
) -> Vec<Section<'a>> {
    let content_name = <modifiers::Content as Modify>::name().unwrap();

    // Either we have a `content` metadata or we re-use section
    let Some(Sections(mut sections)) = content else {
        // TODO(err): might be worth providing an error here
        return match modifiers.iter().position(|m| m.name == content_name) {
            None => vec![],
            Some(index) => {
                let content = modifiers.swap_remove(index).value;
                vec![Section { modifiers, content }]
            }
        }
    };
    // TODO(err)TODO(perf): deduplicate here
    for section in &mut sections {
        section.modifiers.extend(modifiers.clone());
    }
    sections
}
