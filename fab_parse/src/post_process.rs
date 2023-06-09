//! Manipulate the parsed sections

use std::{borrow::Cow, iter, marker::PhantomData, ops::Range, str::FromStr};

use bevy_math::cubic_splines::CubicCurve;
use enumset::{EnumSet, EnumSetType};
use fab::{binding, modify::Modify, resolve::MakeModify, resolve::ModifyKind};
use log::warn;

use crate::tree::{self, get_content, get_content_mut, is_content, Dyn, Hook};

/// Splits the input `Vec` in three, apply `f` on the middle section,
/// creating a new list, and stitches back the `Vec` together.
///
/// ```text
///  range.start --v          range.end --v
/// full:  [a b c d  e f g h i j k l m n o  p q]
/// split: [a b c d][e f g h i j k l m n o][p q]
///         start    middle                 end
///
/// extend = f(&mut start, middle);
///
/// extend:  start   [e f g h z k m] end
/// return: [a b c d  e f g h z k m  p q]
/// ```
#[inline]
fn extend_segment<T>(
    full: Vec<T>,
    middle_range: Range<usize>,
    f: impl FnOnce(&mut [T], Vec<T>) -> Vec<T>,
) -> Vec<T> {
    let mut start = full;
    let mut middle_end = start.split_off(middle_range.start);
    let mut end = middle_end.split_off(middle_range.len());
    let middle = middle_end;

    let mut extended_middle = f(&mut start, middle);

    start.append(&mut extended_middle);
    start.append(&mut end);
    start
}

pub enum Deps<F: EnumSetType> {
    NoneWithName,
    Some {
        changes: EnumSet<F>,
        depends: EnumSet<F>,
    },
}
pub trait Parsable: Modify {
    type Err: Into<anyhow::Error> + Send + Sync;

    fn dependencies_of(name: &str) -> Deps<Self::Field>;
    fn parse(name: &str, value: &str) -> Result<Self, Self::Err>;
}
/// Two strings, one on the left represents the `name` of a modifer,
/// the one on the right represents its `value`.
///
/// Used in the [`TransformedTree::alias`] method.
pub trait StringPair<'a> {
    fn string_pair(self) -> (&'a str, &'a str);
}
impl<'a, 'b: 'a, K: AsRef<str> + 'a, V: AsRef<str> + 'a> StringPair<'a> for &'b (K, V) {
    fn string_pair(self) -> (&'a str, &'a str) {
        (self.0.as_ref(), self.1.as_ref())
    }
}

#[derive(Debug)]
struct Modifier<M> {
    influence: usize,
    inner: M,
}
#[derive(Debug)]
struct Section<'a, M>(tree::Section<'a>, Vec<Modifier<M>>);
impl<'a, M> From<tree::Section<'a>> for Section<'a, M> {
    fn from(value: tree::Section<'a>) -> Self {
        Section(value, Vec::new())
    }
}
impl<'a, M> Section<'a, M> {
    fn get_content(&self) -> Option<&'a str> {
        self.0.modifiers.iter().find_map(get_content)
    }

    fn iter_mut_modifiers(&mut self) -> impl Iterator<Item = &mut tree::Modifier<'a>> + '_ {
        self.0.modifiers.iter_mut().filter(|m| !is_content(m))
    }

    fn increment_mods(&mut self, additional_subsections: usize) {
        for modi in self.iter_mut_modifiers() {
            modi.subsection_count += additional_subsections;
        }
        for modi in self.1.iter_mut() {
            modi.influence += additional_subsections;
        }
    }

    fn set_content(&mut self, new_content: &'a str) -> Option<()> {
        let content = self.0.modifiers.iter_mut().find_map(get_content_mut)?;
        *content = new_content;
        Some(())
    }

    fn increment_exceeding(&mut self, max: usize, increment_by: usize) {
        for modi in self.iter_mut_modifiers() {
            if modi.subsection_count > max {
                modi.subsection_count += increment_by;
            }
        }
        for modi in self.1.iter_mut() {
            if modi.influence > max {
                modi.influence += increment_by;
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Split {
    /// Split the section by word.
    ByWord,

    /// Split the section by character.
    ByChar,
}
impl Split {
    fn iter(self, to_split: &str) -> impl Iterator<Item = &str> {
        use Split::{ByChar, ByWord};
        let split_by = move |c: char| match self {
            ByChar => true,
            // TODO(bug): this doesn't handle nicely multiple sequential spaces
            ByWord => c.is_whitespace(),
        };
        to_split.split_inclusive(split_by)
    }

    fn first(self, to_split: &str) -> (&str, Vec<&str>) {
        let mut iter = self.iter(to_split);
        let head = iter.next().unwrap(); // TODO(err)
        (head, iter.collect())
    }

    fn count<M: Modify>(self, sections: &[Section<M>]) -> usize {
        let count_one = |s: &Section<M>| s.get_content().map_or(0, |m| self.iter(m).count());
        sections.iter().map(|s| count_one(s)).sum()
    }
}

struct Splitter<'a, Mk: FnMut(&str, usize) -> M, M: Modify> {
    split: Split,
    alias: &'a str,
    chopper: Mk,
    _p: PhantomData<M>,
}
impl<'a, Mk, M: Modify> Splitter<'a, Mk, M>
where
    Mk: FnMut(&str, usize) -> M,
{
    fn rmod(&mut self, input: &str, count: usize) -> Modifier<M> {
        Modifier { influence: 1, inner: (self.chopper)(input, count) }
    }
    fn is(&self, modi: &tree::Modifier) -> bool {
        modi.name == self.alias
    }
    fn extract_from<'b>(&self, section: &mut Section<'b, M>) -> Option<(&'b str, usize)> {
        let index = section.0.modifiers.iter().position(|m| self.is(m))?;
        let modifier = section.0.modifiers.remove(index);

        // TODO(err): Result instead
        match modifier.value {
            Dyn::Dynamic(_) => None,
            Dyn::Static(value) => Some((value, modifier.subsection_count)),
        }
    }
    fn process(mut self, mut sections: Vec<Section<M>>) -> Vec<Section<M>> {
        let mut i = 0;
        loop {
            let Some(section) = sections.get_mut(i) else {
                return sections;
            };
            let Some((repeat_value, sub_count)) = self.extract_from(section) else {
                i += 1;
                continue;
            };
            let content_count = self.split.count(&sections[i..i + sub_count]);

            sections = extend_segment(sections, i..i + sub_count, |start, range| {
                let mut replacements = Vec::with_capacity(content_count);
                for mut current in range.into_iter() {
                    let Some(content) = current.get_content() else {
                        replacements.push(current);
                        continue;
                    };
                    let (head, tail) = self.split.first(content);

                    // SAFETY: we `continue` if `current.get_content()` returns None earlier
                    unsafe { current.set_content(head).unwrap_unchecked() };

                    current.increment_mods(tail.len());

                    for (prev_i, section) in start.iter_mut().enumerate() {
                        section.increment_exceeding(i - prev_i, tail.len());
                    }
                    current.1.push(self.rmod(repeat_value, content_count));

                    let tail = tail.into_iter().map(|content| {
                        let rmod = self.rmod(repeat_value, content_count);
                        Section(tree::Section::free(content).unwrap(), vec![rmod])
                    });
                    replacements.extend(iter::once(current).chain(tail));
                }
                replacements
            });
            i += 1;
        }
    }
}

/// A format string's sections, parsed but still can be manipulated through styles.
///
/// See methods on this `struct` for more details on what kind of transforms apply.
pub struct Styleable<'a, M> {
    sections: Vec<Section<'a, M>>,
}
impl<'a> tree::Tree<'a> {
    pub fn transform<M: Parsable>(self) -> Styleable<'a, M> {
        Styleable::new(self.sections)
    }
}
impl<'a, M: Parsable> Styleable<'a, M> {
    pub(super) fn new(sections: Vec<tree::Section<'a>>) -> Self {
        let max_range = |s: &tree::Section| s.modifiers.iter().map(|m| m.subsection_count).max();
        let max_sect = |(i, s): (usize, _)| max_range(s).unwrap_or(0) + i;
        let max_sect = sections.iter().enumerate().map(max_sect).max();
        let max_sect = max_sect.unwrap_or(0);

        assert!(max_sect < u32::MAX as usize, "Too many sections! over 2³²");

        Styleable {
            sections: sections.into_iter().map(Section::from).collect(),
        }
    }
    pub fn finish(
        self,
        bindings: &mut binding::World<M>,
        hooks: &mut Vec<Hook<'a>>,
        // TODO encapsulate MakeModify<M>
    ) -> Vec<anyhow::Result<MakeModify<M>>> {
        let sections = self.sections;
        let mut to_modify_kind = |name, value| match value {
            Dyn::Dynamic(target) => {
                let binding = bindings.get_or_add(target.path.binding());
                if let Some(hook) = target.as_hook() {
                    hooks.push(hook);
                }
                let Deps::Some{ depends, changes } = M::dependencies_of(name) else {
                    return Err(anyhow::anyhow!(format!("{name} is not a modifier")));
                };
                Ok(ModifyKind::Bound { binding, depends, changes })
            }
            Dyn::Static(value) => {
                let mut value = value.into();
                escape_backslashes(&mut value);
                let parsed = M::parse(name, &value).map_err(|t| t.into())?;
                Ok(ModifyKind::Modify(parsed))
            }
        };
        let mut to_make_modify = |i, tree::Modifier { name, value, subsection_count }| {
            Ok(MakeModify {
                range: i as u32..(i + subsection_count) as u32,
                kind: to_modify_kind(name, value)?,
            })
        };
        let to_make_rmodify = |i, Modifier { influence, inner }| {
            Ok(MakeModify {
                range: i as u32..(i + influence) as u32,
                kind: ModifyKind::Modify(inner),
            })
        };
        sections
            .into_iter()
            .enumerate()
            .flat_map(|(i, Section(native, extend))| {
                let to_make_modify = |m| to_make_modify(i, m);
                let to_make_rmodify = move |m| to_make_rmodify(i, m);

                let native = native.modifiers.into_iter().map(to_make_modify);
                let extend = extend.into_iter().map(to_make_rmodify);
                // TODO(perf): This colllect solves a particularly tricky lifetime issue,
                // not sure if it is possible to do without
                native.chain(extend).collect::<Vec<_>>()
            })
            .collect()
    }
}
/// Add aliases
impl<'a, M: Modify> Styleable<'a, M> {
    /// Replace all occurences of modifier named `alias` with the output of
    /// `producers`, as pure text values.
    ///
    /// The input value of `producers` is the value (right side of `modifier: value`)
    /// of the modifier with given alias.
    ///
    /// Note that the generated modifiers will be appended at the end of the
    /// modifiers of this section, and keep the same section range.
    ///
    /// # Example
    ///
    /// The lifetimes are a bit tricky here, hopefully they work out for you!
    ///
    /// Note that the return value of `producer` is `I: IntoIterator<Item = Sp>`,
    /// meaning the following should work:
    ///
    /// ```no_run
    /// # use cuicui_fab::__private::DummyModify;
    /// # use cuicui_fab_parse::TransformedTree;
    /// # // This would be unsoud only if this code ran.
    /// # let transformed_tree: TransformedTree<DummyModify> = unsafe {
    /// #     std::mem::MaybeUninit::uninit().assume_init() };
    /// let aliased_bleepo = transformed_tree.alias("Bleepoo", |_| [
    ///   ("Zooba", "whoop_whoop"),
    ///   ("Bazinga", "whaab"),
    ///   ("Bilyboo", "tarumba"),
    /// ]);
    /// // Now, all occurences of Bleepo will be replaced with given modifiers
    /// ```
    pub fn alias<Sp: StringPair<'a>, I: IntoIterator<Item = Sp>>(
        mut self,
        alias: &str,
        mut producer: impl FnMut(&str) -> I,
    ) -> Self {
        for section in &mut self.sections {
            let mut extensions = Vec::new();
            let mut iter = section.0.modifiers.iter();
            if let Some(modifier) = iter.find(|m| m.name == alias) {
                if let Dyn::Static(value) = modifier.value {
                    let new_modifiers =
                        producer(value)
                            .into_iter()
                            .map(Sp::string_pair)
                            .map(|(name, value)| tree::Modifier {
                                name,
                                value: Dyn::Static(value),
                                subsection_count: modifier.subsection_count,
                            });
                    extensions.extend(new_modifiers);
                } else {
                    warn!("alias {alias} had a bound value, this isn't supported",);
                }
            }
            section.0.modifiers.retain(|m| m.name != alias);
            section.0.modifiers.append(&mut extensions);
        }
        Styleable { sections: self.sections }
    }
    /// Replace all occurences of modifier named `alias` with the output of
    /// `producers`.
    ///
    /// The input value of `producers` is the value (right side of `modifier: value`)
    /// of the modifier with given alias.
    ///
    /// Note that the generated modifiers will be appended at the end of the
    /// modifiers of this section, and keep the same section range.
    pub fn alias_mods<I: IntoIterator<Item = M>>(
        mut self,
        alias: &str,
        mut producer: impl FnMut(&str) -> I,
    ) -> Self {
        for section in &mut self.sections {
            let mut extensions = Vec::new();
            let mut iter = section.0.modifiers.iter();
            if let Some(modifier) = iter.find(|m| m.name == alias) {
                if let Dyn::Static(value) = modifier.value {
                    let mk_modifier =
                        |inner| Modifier { inner, influence: modifier.subsection_count };
                    extensions.extend(producer(value).into_iter().map(mk_modifier));
                } else {
                    warn!("alias {alias} had a bound value, this isn't supported",);
                }
            }
            section.0.modifiers.retain(|m| m.name != alias);
            section.1.append(&mut extensions);
        }
        Styleable { sections: self.sections }
    }
}
/// Cut the tree in various ways
impl<'a, M: Modify> Styleable<'a, M> {
    pub fn chop(self, split: Split, alias: &str, chopper: impl FnMut(&str, usize) -> M) -> Self {
        let split = Splitter { split, alias, chopper, _p: PhantomData };
        Styleable { sections: split.process(self.sections) }
    }
    pub fn acc_chop<Acc: FromStr>(
        self,
        split: Split,
        alias: &str,
        mut chopper: impl FnMut(&mut Acc, usize, usize) -> M,
    ) -> Self {
        let mut acc = None;
        let mut i = 0;

        self.chop(split, alias, move |input: &str, count| {
            let acc = acc.get_or_insert_with(|| input.parse::<Acc>().ok().unwrap());
            let result = chopper(acc, i, count);
            i += 1;
            result
        })
    }
    pub fn curve_chop<Acc: FromStr>(
        self,
        split: Split,
        alias: &str,
        spline: CubicCurve<f32>,
        mut chopper: impl FnMut(&mut Acc, f32) -> M,
    ) -> Self {
        let segment_count = spline.iter_samples(1, |_, i| i).last().unwrap();

        self.acc_chop(split, alias, move |acc, i, count| {
            let step = segment_count / count as f32;
            let position = spline.position(i as f32 * step);
            chopper(acc, position)
        })
    }
}
fn escape_backslashes(input: &mut Cow<str>) {
    if !input.contains('\\') {
        return;
    }
    let input = input.to_mut();
    let mut prev_normal = true;
    input.retain(|c| {
        let backslash = c == '\\';
        let remove = prev_normal && backslash;
        let normal = !remove;
        prev_normal = normal || !backslash;
        normal
    });
}
