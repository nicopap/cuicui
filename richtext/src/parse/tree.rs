//! Manipulate the parsed sections

use std::{borrow::Cow, iter, str::FromStr};

use bevy::math::cubic_splines::CubicCurve;
use fab::resolve::{MakeModify, ModifyKind};

use super::structs::{get_content, get_content_mut, is_content, Dyn, Hook, Modifier, Section};
use crate::{modifiers::Modifier as TextModifier, richtext::TextPrefab, WorldBindings};

#[derive(Debug)]
struct RModifier {
    influence: usize,
    inner: TextModifier,
}
#[derive(Debug)]
pub struct RSection<'a>(Section<'a>, Vec<RModifier>);
impl<'a> From<Section<'a>> for RSection<'a> {
    fn from(value: Section<'a>) -> Self {
        RSection(value, Vec::new())
    }
}
impl<'a> RSection<'a> {
    fn get_content(&self) -> Option<&'a str> {
        self.0.modifiers.iter().find_map(get_content)
    }

    fn iter_mut_modifiers(&mut self) -> impl Iterator<Item = &mut Modifier<'a>> + '_ {
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
pub enum Repeat {
    /// Split the section by word.
    ByWord,

    /// Split the section by character.
    ByChar,
}
impl Repeat {
    fn split_iter(self, to_split: &str) -> impl Iterator<Item = &str> {
        use Repeat::{ByChar, ByWord};
        let split_by = move |c: char| match self {
            ByChar => true,
            // TODO(bug): this doesn't handle nicely multiple sequential spaces
            ByWord => c.is_whitespace(),
        };
        to_split.split_inclusive(split_by)
    }
    fn split(self, to_split: &str) -> (&str, Vec<&str>) {
        let mut iter = self.split_iter(to_split);
        let head = iter.next().unwrap(); // TODO(err)
        (head, iter.collect())
    }
    fn count_one(self, section: &RSection) -> usize {
        section
            .get_content()
            .map_or(0, |m| self.split_iter(m).count())
    }
    fn count(self, sections: &[RSection]) -> usize {
        sections.iter().map(|s| self.count_one(s)).sum()
    }
}

struct Splitter<M: FnMut(&str, usize) -> TextModifier> {
    repeat: Repeat,
    alias: Box<str>,
    mk_modifier: M,
}
impl<M: FnMut(&str, usize) -> TextModifier> Splitter<M> {
    fn rmod(&mut self, input: &str, count: usize) -> RModifier {
        RModifier {
            influence: 1,
            inner: (self.mk_modifier)(input, count),
        }
    }
    fn is(&self, modi: &Modifier) -> bool {
        modi.name == self.alias.as_ref()
    }
    fn extract_from<'a>(&self, section: &mut RSection<'a>) -> Option<(&'a str, usize)> {
        let index = section.0.modifiers.iter().position(|m| self.is(m))?;
        let modifier = section.0.modifiers.remove(index);

        // TODO(err): Result instead
        match modifier.value {
            Dyn::Dynamic(_) => None,
            Dyn::Static(value) => Some((value, modifier.subsection_count)),
        }
    }
    fn split<'a>(&mut self, mut sections: Vec<RSection<'a>>) -> Vec<RSection<'a>> {
        let mut i = 0;
        loop {
            let Some(section) = sections.get_mut(i) else {
                return sections;
            };
            let Some((repeat_value, sub_count)) = self.extract_from(section) else {
                i += 1;
                continue;
            };
            let content_count = self.repeat.count(&sections[i..i + sub_count]);

            let mut prev_sections = sections;
            let mut affected_sections = prev_sections.split_off(i);
            sections = affected_sections.split_off(sub_count);

            let mut replacements = Vec::with_capacity(content_count);

            for mut current in affected_sections.into_iter() {
                let content = current.get_content().unwrap(); // TODO(err)
                let (head, tail) = self.repeat.split(content);

                current.set_content(head).unwrap(); // TODO(err): (this should never be None)
                current.increment_mods(tail.len());

                for (prev_i, section) in prev_sections.iter_mut().enumerate() {
                    section.increment_exceeding(i - prev_i, tail.len());
                }
                current.1.push(self.rmod(repeat_value, content_count));

                let tail = tail.into_iter().map(|content| {
                    let rmod = self.rmod(repeat_value, content_count);
                    RSection(Section::free(content).unwrap(), vec![rmod])
                });
                replacements.extend(iter::once(current).chain(tail));
            }
            prev_sections.append(&mut replacements);
            prev_sections.append(&mut sections);
            sections = prev_sections;
            i += 1;
        }
    }
}

mod sealed {
    pub trait Split {}
}
impl sealed::Split for () {}
impl<M: FnMut(&str, usize) -> TextModifier, S> sealed::Split for (Splitter<M>, S) {}

pub trait Split: sealed::Split {
    fn all_splits<'a>(&mut self, sections: Vec<RSection<'a>>) -> Vec<RSection<'a>>;
}
impl Split for () {
    #[inline]
    fn all_splits<'a>(&mut self, sections: Vec<RSection<'a>>) -> Vec<RSection<'a>> {
        sections
    }
}
impl<MkMod: FnMut(&str, usize) -> TextModifier, S: Split> Split for (Splitter<MkMod>, S) {
    #[inline]
    fn all_splits<'a>(&mut self, mut sections: Vec<RSection<'a>>) -> Vec<RSection<'a>> {
        let (current, tail) = self;
        sections = current.split(sections);
        tail.all_splits(sections)
    }
}
pub struct TreeSplitter<S: Split>(S);
impl TreeSplitter<()> {
    pub fn new() -> Self {
        TreeSplitter(())
    }
}
impl<S: Split> TreeSplitter<S> {
    pub fn repeat_acc<Acc: FromStr>(
        self,
        repeat: Repeat,
        alias: impl Into<Box<str>>,
        mut mk_modifier: impl FnMut(&mut Acc, usize, usize) -> TextModifier,
    ) -> TreeSplitter<impl Split> {
        let mut acc = None;
        let mut i = 0;
        self.repeat(repeat, alias, move |input: &str, count| {
            let acc = acc.get_or_insert_with(|| input.parse::<Acc>().ok().unwrap());
            let result = mk_modifier(acc, i, count);
            i += 1;
            result
        })
    }
    pub fn repeat(
        self,
        repeat: Repeat,
        alias: impl Into<Box<str>>,
        mk_modifier: impl FnMut(&str, usize) -> TextModifier,
    ) -> TreeSplitter<impl Split> {
        let new_split = Splitter { repeat, alias: alias.into(), mk_modifier };
        TreeSplitter((new_split, self.0))
    }
    pub fn repeat_on_curve<Acc: FromStr>(
        self,
        repeat: Repeat,
        alias: impl Into<Box<str>>,
        spline: CubicCurve<f32>,
        mut mk_modifier: impl FnMut(&mut Acc, f32) -> TextModifier,
    ) -> TreeSplitter<impl Split> {
        let segment_count = spline.iter_samples(1, |_, i| i).last().unwrap();
        self.repeat_acc(repeat, alias, move |acc, i, count| {
            let step = segment_count / count as f32;
            let position = spline.position(i as f32 * step);
            mk_modifier(acc, position)
        })
    }
}
pub struct Tree<'a> {
    sections: Vec<RSection<'a>>,
}
impl<'a> Tree<'a> {
    pub(super) fn new(sections: Vec<Section<'a>>) -> Self {
        Tree {
            sections: sections.into_iter().map(RSection::from).collect(),
        }
    }
    pub fn split(self, mut splitter: TreeSplitter<impl Split>) -> Self {
        Tree { sections: splitter.0.all_splits(self.sections) }
    }
    pub fn parse(
        self,
        bindings: &mut WorldBindings,
        hooks: &mut Vec<Hook<'a>>,
    ) -> Vec<anyhow::Result<MakeModify<TextPrefab>>> {
        foo(self.sections, bindings, hooks)
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
fn foo<'a>(
    sections: Vec<RSection<'a>>,
    bindings: &mut WorldBindings,
    hooks: &mut Vec<Hook<'a>>,
) -> Vec<anyhow::Result<MakeModify<TextPrefab>>> {
    let mut to_modify_kind = |name, value| match value {
        Dyn::Dynamic(target) => {
            let binding = bindings.intern(target.path.binding());
            if let Some(hook) = target.as_hook() {
                hooks.push(hook);
            }
            Ok(ModifyKind::Bound(binding))
        }
        Dyn::Static(value) => {
            let mut value = value.into();
            escape_backslashes(&mut value);
            TextModifier::parse(name, &value).map(ModifyKind::Modify)
        }
    };
    let try_u32 = u32::try_from;
    let mut to_make_modify = |i, Modifier { name, value, subsection_count }| {
        Ok(MakeModify {
            range: try_u32(i)?..try_u32(i + subsection_count)?,
            kind: to_modify_kind(name, value)?,
        })
    };
    let to_make_rmodify = |i, RModifier { influence, inner }| {
        Ok(MakeModify {
            range: try_u32(i)?..try_u32(i + influence)?,
            kind: ModifyKind::Modify(inner),
        })
    };
    sections
        .into_iter()
        .enumerate()
        .flat_map(|(i, RSection(native, extend))| {
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
