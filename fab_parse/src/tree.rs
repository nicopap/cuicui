//! Intermediate parsing representation.

use crate::rt_fmt::RuntimeFormat;

use winnow::stream::Accumulate;

pub(crate) const CONTENT_NAME: &str = "Content";
pub(crate) fn is_content(m: &Modifier) -> bool {
    m.name == CONTENT_NAME
}
pub(crate) fn get_content<'a>(m: &Modifier<'a>) -> Option<&'a str> {
    match m.value {
        Dyn::Static(value) if is_content(m) => Some(value),
        _ => None,
    }
}
pub(crate) fn get_content_mut<'a, 'b>(m: &'b mut Modifier<'a>) -> Option<&'b mut &'a str> {
    let is_content = is_content(m);
    match &mut m.value {
        Dyn::Static(value) if is_content => Some(value),
        _ => None,
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Path<'a> {
    Binding(&'a str),
    Tracked(Source<'a>),
}
impl<'a> Path<'a> {
    pub(crate) fn binding(&self) -> &'a str {
        use Path::*;

        let (Binding(binding) | Tracked(Source { binding, .. })) = self;
        binding
    }
}
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Source<'a> {
    pub query: Query<'a>,
    pub reflect_path: &'a str,
    /// Full name of the binding. This is query + reflect_path
    pub binding: &'a str,
}
impl<'a> Source<'a> {
    pub(crate) fn new(((query, reflect_path), binding): ((Query<'a>, &'a str), &'a str)) -> Self {
        Source { query, reflect_path, binding }
    }
}

/// Where to pull from the value.
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Query<'a> {
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
    pub(crate) fn name((name, access): (&'a str, &'a str)) -> Self {
        Query::Name { name, access }
    }
    pub(crate) fn marked((marker, access): (&'a str, &'a str)) -> Self {
        Query::Marked { marker, access }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Section<'a> {
    pub(crate) modifiers: Vec<Modifier<'a>>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) struct Modifier<'a> {
    pub(crate) name: &'a str,
    pub(crate) value: Dyn<'a>,
    pub(crate) subsection_count: usize,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum Dyn<'a> {
    Dynamic(Binding<'a>),
    Static(&'a str),
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Hook<'a> {
    pub source: Source<'a>,
    pub format: Option<Format<'a>>,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub(crate) struct Binding<'a> {
    pub(crate) path: Path<'a>,
    pub(crate) format: Option<Format<'a>>,
}
impl<'a> Binding<'a> {
    #[cfg(test)]
    pub(crate) fn named(name: &'a str) -> Self {
        Binding { path: Path::Binding(name), format: None }
    }

    pub(crate) fn as_hook(&self) -> Option<Hook<'a>> {
        if let Path::Tracked(source) = self.path {
            Some(Hook { source, format: self.format })
        } else {
            None
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Format<'a> {
    UserDefined(&'a str),
    Fmt(RuntimeFormat),
}

/// Accumulate many sections.
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Sections<'a>(pub(crate) Vec<Section<'a>>);
impl<'a> Sections<'a> {
    pub(crate) fn full_subsection((head, mut tail): (Option<Section<'a>>, Self)) -> Self {
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
    pub(crate) fn free(input: &'a str) -> Option<Self> {
        if input.is_empty() {
            return None;
        }
        let modifier = Modifier::new((CONTENT_NAME, Dyn::Static(input)));
        Some(Section { modifiers: vec![modifier] })
    }
    /// A delimited section (ie between {}).
    pub(crate) fn format(input: Binding<'a>) -> Vec<Self> {
        let modifier = Modifier::new((CONTENT_NAME, Dyn::Dynamic(input)));
        vec![Section { modifiers: vec![modifier] }]
    }
}

impl<'a> Modifier<'a> {
    pub(crate) fn new((name, value): (&'a str, Dyn<'a>)) -> Self {
        Self { name, value, subsection_count: 1 }
    }
}

impl<'a> Binding<'a> {
    pub(crate) fn format((path, format): (Path<'a>, Option<Format<'a>>)) -> Self {
        Binding { path, format }
    }
}

pub struct Tree<'a> {
    pub(crate) sections: Vec<Section<'a>>,
}

/// Create a section with given `modifers`
///
/// ```text
/// {Modifier1: 1.32, Modifier2: hi|Some text{a_subsection}and more{Modifier3:34.3|subsections}}
/// ```
///
/// Here, the `modifiers` would be:
/// ```text
/// [ Modifier1: 1.32, Modifier2: hi]
/// ```
///
/// `content` would be:
/// ```text
/// [
///     Some text
///     {a_subsection}
///     and more
///     {Modifier3: 34.3[subsections]}
/// ]
/// ```
///
/// Applying `flatten_section` stores the modifiers in the first section.
/// The `subsection_count` of each modifier is increased, this tells the rest
/// of the code that the modifier affects not only this first section, but also
/// all `subsection_count` other subsections existing afterward (including the first).
///
/// `flatten_section` will return a `Vec<Section>` with the first `Section`
/// containing all the `modifiers`, where `subsection_count` is how many subsections
/// there are.
///
/// A typical section doesn't have subsections:
/// ```text
/// {Modifier1: 1.32 |This is a typical section}
/// ```
///
/// This produces a `Vec<Section>` with a single element.
pub(crate) fn flatten_section<'a>(
    (mut modifiers, content): (Vec<Modifier<'a>>, Option<Sections<'a>>),
) -> Vec<Section<'a>> {
    // Either we have a `content` metadata or we re-use section
    let mut sections = if let Some(Sections(sections)) = content {
        sections
    } else if modifiers.iter().any(is_content) {
        vec![Section { modifiers: Vec::with_capacity(modifiers.len()) }]
    } else {
        // TODO(err): should error here, we have metadata and no content,
        // this discard something the user wrote, means they probably didn't
        // intend on this behavior.
        return vec![];
    };
    let subsection_count = sections.len();

    // TODO(err): verify that we never have duplicate CONTENT_NAME
    if let Some(first_section) = sections.get_mut(0) {
        let extended_modifiers = modifiers
            .drain(..)
            .map(|m| Modifier { subsection_count, ..m });
        first_section.modifiers.extend(extended_modifiers);
    }
    sections
}
