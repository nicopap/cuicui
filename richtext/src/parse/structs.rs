//! Intermediate parsing representation.

use crate::show::RuntimeFormat;

use winnow::stream::Accumulate;

const CONTENT_NAME: &str = "Content";

#[derive(PartialEq, Debug, Clone, Copy)]
pub(super) enum Path<'a> {
    Binding(&'a str),
    Tracked(Source<'a>),
}
impl<'a> Path<'a> {
    pub(super) fn binding(&self) -> &'a str {
        use Path::*;

        let (Binding(binding) | Tracked(Source{ binding, .. })) = self;
        binding
    }
}
#[derive(PartialEq, Debug, Clone, Copy)]
pub(crate) struct Source<'a>{
    pub(crate) query: Query<'a>,
    pub(crate) reflect_path: &'a str,
    /// Full name of the binding. This is query + reflect_path
    pub(crate) binding: &'a str,
}
impl<'a> Source<'a> {
    pub(super) fn new(((query, reflect_path), binding): ((Query<'a>, &'a str), &'a str)) -> Self {
        Source { query, reflect_path, binding }
    }
}

/// Where to pull from the value.
#[derive(PartialEq, Debug, Clone, Copy)]
pub(crate) enum Query<'a> {
    /// A [`Resource`] implementing [`Reflect`].
    Res(&'a str),
    /// The first [`Entity`] found with provided component.
    One(&'a str),
    /// The first [`Entity`] found with the given name
    Name { name: &'a str, access: &'a str },
    /// The first [`Entity`] found with provided component, but access a
    /// different component.
    Marked { marker: &'a str, access: &'a str },
}
impl<'a> Query<'a> {
    pub(super) fn name((name, access): (&'a str, &'a str)) -> Self {
        Query::Name { name, access }
    }
    pub(super) fn marked((marker, access): (&'a str, &'a str)) -> Self {
        Query::Marked { marker, access }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(super) struct Section<'a> {
    pub(super) modifiers: Vec<Modifier<'a>>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(super) struct Modifier<'a> {
    pub(super) name: &'a str,
    pub(super) value: Dyn<'a>,
    pub(super) subsection_count: usize,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(super) enum Dyn<'a> {
    Dynamic(Binding<'a>),
    Static(&'a str),
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Hook<'a> {
    pub(crate) source: Source<'a>,
    pub(crate) format: Option<Format<'a>>,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub(super) struct Binding<'a> {
    pub(super) path: Path<'a>,
    pub(super) format: Option<Format<'a>>,
}
impl<'a> Binding<'a> {
    #[cfg(test)]
    pub(super) fn named(name: &'a str) -> Self {
        Binding {
            path: Path::Binding(name),
            format: None,
        }
    }

    pub(crate) fn as_pull(&self) -> Option<Hook<'a>> {
        if let Path::Tracked(source) = self.path {
            Some(Hook { source, format: self.format})
        } else {
            None
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub(crate) enum Format<'a> {
    UserDefined(&'a str),
    Fmt(RuntimeFormat),
}

/// Accumulate many sections.
#[derive(Debug, PartialEq, Clone)]
pub(super) struct Sections<'a>(pub(super) Vec<Section<'a>>);
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

impl<'a> Section<'a> {
    pub(super) fn free(input: &'a str) -> Option<Self> {
        if input.is_empty() {
            return None;
        }
        let modifier = Modifier::new((CONTENT_NAME, Dyn::Static(input)));
        Some(Section { modifiers: vec![modifier] })
    }
    /// A delimited section (ie between {}).
    pub(super) fn format(input: Binding<'a>) -> Vec<Self> {
        let modifier = Modifier::new((CONTENT_NAME, Dyn::Dynamic(input)));
        vec![Section { modifiers: vec![modifier] }]
    }
}

impl<'a> Modifier<'a> {
    pub(super) fn new((name, value): (&'a str, Dyn<'a>)) -> Self {
        Self { name, value, subsection_count: 1 }
    }
}

impl<'a> Binding<'a> {
    pub(super) fn format((path, format): (Path<'a>, Option<Format<'a>>)) -> Self {
        Binding { path, format }
    }
}

pub(super) fn flatten_section<'a>(
    (mut modifiers, content): (Vec<Modifier<'a>>, Option<Sections<'a>>),
) -> Vec<Section<'a>> {
    // Either we have a `content` metadata or we re-use section
    let Some(Sections(mut sections)) = content else {
        return match modifiers.iter().find(|m| m.name == CONTENT_NAME) {
            // TODO(err): should error here, we have metadata and no content
            None => vec![],
            Some(_) => vec![Section { modifiers }],
            
        }
    };
    let subsection_count = sections.len();

    // TODO(err): verify that we never have duplicate CONTENT_NAME
    if let Some(first_section) = sections.get_mut(0) {
        let extended_modifiers = modifiers.drain(..).map(|m| Modifier { subsection_count,..m});
        first_section.modifiers.extend(extended_modifiers);
    }
    sections
}

