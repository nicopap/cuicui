mod make;

use std::{fmt, iter, ops::Range};

use bevy::{
    prelude::warn,
    reflect::{Reflect, Typed},
    text::{BreakLineOn, Text, TextAlignment, TextStyle},
    utils::HashMap,
};
use datazoo::{sorted, BitMultiMap, EnumBitMatrix, EnumMultiMap, SortedIterator};
use enumset::{EnumSet, __internal::EnumSetTypePrivate};

use crate::{
    change_text::ChangeTextStyle, modify::BindingId, modify::Change, modify::GetFont, parse,
    parse::interpret, show, show::ShowBox, track::Tracker, AnyError, BindingsView, ModifyBox,
};

/// Index in `modifies`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ModifyIndex(u32);
impl fmt::Debug for ModifyIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<M{}>", self.0)
    }
}

#[derive(Debug)]
pub struct RichText {
    modifies: Box<[Modifier]>,

    /// `Modify` that can be triggered by a context change
    direct_deps: EnumMultiMap<Change, ModifyIndex, { (Change::BIT_WIDTH - 1) as usize }>,

    // TODO(feat): RichText without m2m dependency. This is fairly costly to
    // build and uses several kilobytes of memory.
    /// `Modify` that depends on other `Modify`.
    ///
    /// When a `Modify` changes, sometimes, other `Modify` need to run.
    modify_deps: BitMultiMap<ModifyIndex, ModifyIndex>,

    // TODO(feat): Multiple bindings, see `nested_modify.md#bindings-representation`
    /// Binding ranges.
    ///
    /// Note that this prevents having 1 binding to N instances.
    bindings: sorted::ByKeyBox<BindingId, Range<u32>>,

    root_mask: EnumBitMatrix<Change>,
}

struct RichTextCtx<'a> {
    style: &'a TextStyle,
    rich: &'a RichText,
    get_font: GetFont<'a>,
    to_update: &'a mut Text,
}

impl<'a> RichTextCtx<'a> {
    fn apply_modify(
        &mut self,
        index: ModifyIndex,
        mask: impl SortedIterator<Item = u32>,
        // TODO(clean)
        should_reset_style: bool,
    ) -> Result<(), AnyError> {
        let Modifier { modify, range } = self.rich.modify_at(index);

        for section in range.clone().difference(mask) {
            let section = &mut self.to_update.sections[section as usize];
            if should_reset_style {
                section.style = self.style.clone();
            }
            modify.apply(&self.get_font, section)?;
        }
        Ok(())
    }
    fn apply_modify_deps<I>(&mut self, index: ModifyIndex, mask: impl Fn() -> I)
    where
        I: SortedIterator<Item = u32>,
    {
        if let Err(err) = self.apply_modify(index, mask(), true) {
            warn!("when applying {index:?}: {err}");
        }
        for &dep_index in self.rich.modify_deps.get(&index) {
            if let Err(err) = self.apply_modify(dep_index, mask(), false) {
                warn!("when applying {dep_index:?} child of {index:?}: {err}");
            }
        }
    }
    pub fn update<'b>(
        &mut self,
        changes: EnumSet<Change>,
        bindings: impl Iterator<Item = (BindingId, &'b ModifyBox)>,
    ) {
        for (id, modify) in bindings {
            let Some((_, range)) = self.rich.binding_range(id) else {
                continue;
            };
            for section in range.difference(iter::empty()) {
                let section = &mut self.to_update.sections[section as usize];
                if let Err(err) = modify.apply(&self.get_font, section) {
                    warn!("when applying {id:?}: {err}");
                }
            }
        }
        for index in self.rich.change_modifies(changes) {
            let Modifier { modify, range } = self.rich.modify_at(index);
            let mask = || self.rich.root_mask_for(modify.changes(), range.clone());
            self.apply_modify_deps(index, mask);
        }
    }
}
impl RichText {
    fn binding_range(&self, binding: BindingId) -> Option<(usize, Range<u32>)> {
        // TODO(perf): binary search THE FIRST binding, then `intersected`
        // the slice from it to end of `dynamic` with the sorted Iterator of BindingId.
        let index = self.bindings.binary_search_by_key(&binding, |d| d.0).ok()?;
        Some((index, self.bindings[index].1.clone()))
    }
    fn change_modifies(&self, changes: EnumSet<Change>) -> impl Iterator<Item = ModifyIndex> + '_ {
        self.direct_deps.all_rows(changes).copied()
    }
    fn root_mask_for(
        &self,
        _changes: EnumSet<Change>,
        _range: Range<u32>,
    ) -> impl SortedIterator<Item = u32> + '_ {
        // TODO(bug): need to get all change masks and merge them
        iter::empty()
    }
    fn modify_at(&self, index: ModifyIndex) -> &Modifier {
        // SAFETY: we kinda assume that it is not possible to build an invalid `ModifyIndex`.
        unsafe { self.modifies.get_unchecked(index.0 as usize) }
    }
    pub fn update<'a>(
        &'a self,
        to_update: &'a mut Text,
        style_changes: &'a ChangeTextStyle,
        bindings: BindingsView<'a>,
        get_font: GetFont<'a>,
    ) {
        let bindings = bindings.changed();
        let style = &style_changes.inner;
        let changes = style_changes.changes;
        RichTextCtx { rich: self, get_font, to_update, style }.update(changes, bindings);
    }
}

#[derive(Debug)]
pub(crate) enum ModifyKind {
    Bound(BindingId),
    Modify(ModifyBox),
}
#[derive(Debug)]
pub(crate) struct ParseModifier {
    pub(crate) kind: ModifyKind,
    pub(crate) range: Range<u32>,
}

/// A [`ModifyBox`] that apply to a given [`Range`] of [`TextSection`]s on a [`Text`].
#[derive(Debug)]
struct Modifier {
    /// The modifier to apply in the given `range`.
    modify: ModifyBox,

    /// The range to which to apply the `modify`.
    range: Range<u32>,
}
pub struct RichTextBuilder<'a> {
    pub format_string: String,
    pub(crate) context: interpret::Context<'a>,

    pub parent_style: TextStyle,
    pub get_font: GetFont<'a>,
    pub alignment: TextAlignment,
    pub linebreak_behaviour: BreakLineOn,

    // TODO(perf): This sucks, the `FetchBox`, which we are using this for, is
    // calling itself the `ShowBox` impl. Instead of storing formatters, we should
    // directly construct the `FetchBox` when it is added
    // TODO(feat): This is actually unused.
    pub formatters: HashMap<&'static str, ShowBox>,
}
impl<'a> RichTextBuilder<'a> {
    /// Add a [formatter](crate::show::Show).
    pub fn fmt<I, O, F>(mut self, name: &'static str, convert: F) -> Self
    where
        I: Reflect + Typed,
        O: fmt::Display + 'static, // TODO(bug): shouldn't need this + 'static
        F: Clone + Send + Sync + Fn(&I) -> O + 'static,
    {
        self.formatters
            .insert(name, show::Convert::<I, O, F>::new(convert));
        self
    }
    pub fn build(self) -> Result<(Text, RichText, Vec<Tracker>), AnyError> {
        let Self { format_string, mut context, parent_style, .. } = self;
        let mut trackers = Vec::new();
        let modifiers = parse::richtext(&mut context, &format_string, &mut trackers)?;

        let (sections, rich_text) = make::Make::new(modifiers, parent_style).build(self.get_font);
        let text = Text {
            sections,
            alignment: self.alignment,
            linebreak_behaviour: self.linebreak_behaviour,
        };

        // debug!("Making RichText: {format_string:?}");
        // partial.print_bindings();
        Ok((text, rich_text, trackers))
    }
}
