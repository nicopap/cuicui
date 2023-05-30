//! Create a dependency tree

use std::fmt;

use datazoo::sorted::KeySorted;
use datazoo::{enum_multimap, AssumeSortedByKeyExt, BitMultiMap, EnumBitMatrix};
use enumset::EnumSet;
use log::trace;
use smallvec::SmallVec;

use crate::binding::Id;
use crate::prefab::{FieldsOf, Modify, Prefab, PrefabContext, PrefabField};

use super::{MakeModify, ModifyIndex as Idx, ModifyKind, Resolver};

pub(super) struct Make<'a, P: Prefab> {
    default_section: &'a P::Item,
    modifiers: Vec<super::MakeModify<P>>,
    errors: Vec<anyhow::Error>,
}
// Manual `impl` because we don't want `Make: Debug where P: Debug`, only
// `Make: Debug where P::Item: Debug, PrefabField<P>: Debug`
impl<P: Prefab> fmt::Debug for Make<'_, P>
where
    PrefabField<P>: fmt::Debug,
    P::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Make")
            .field("default_section", &self.default_section)
            .field("modifiers", &self.modifiers)
            .field("errors", &self.errors)
            .finish()
    }
}

impl<'a, P: Prefab> Make<'a, P>
where
    P::Item: Clone + fmt::Debug,
    PrefabField<P>: fmt::Debug,
{
    /// Initialize a [`Make`] to create a [`Resolver`] using [`Make::build`].
    ///
    /// # Limitations
    ///
    /// - All [`Modify::changes`] of `modifiers` **must** be a subset of [`Modify::depends`].
    /// - [`Modify::depends`] may have exactly 1 or 0 components.
    pub(super) fn new(modifiers: Vec<MakeModify<P>>, default_section: &'a P::Item) -> Self {
        Self { modifiers, default_section, errors: Vec::new() }
    }
    fn field_root_mask(&self, field: PrefabField<P>) -> impl Iterator<Item = u32> + '_ {
        let no_deps = move |modify: &&MakeModify<P>| {
            !modify.depends().contains(field) && modify.changes().contains(field)
        };
        self.modifiers
            .iter()
            .filter(no_deps)
            .flat_map(|modify| modify.range.clone())
    }
    /// The mask of static sections.
    ///
    /// If the bit is enabled, then it shouldn't be updated.
    fn root_mask(&self) -> EnumBitMatrix<PrefabField<P>> {
        let section_count = self.modifiers.iter().map(|m| m.range.end).max();
        let mut root_mask = EnumBitMatrix::new(section_count.unwrap_or(0));

        for field in EnumSet::ALL {
            root_mask.set_row(field, self.field_root_mask(field));
        }
        root_mask
    }
    /// Apply all static `Modify` and remove them from `modifiers`.
    ///
    /// A `Modify` is static when:
    /// - Either depends on nothing or depends on the
    ///    output of a static modifier.  
    /// - For all its items of influences, the field it changes are changed
    ///    by a static modifier child of itself. (TODO(pref) this isn't done yet)
    ///
    /// Note that if there is no `modifiers`, this does nothing.
    fn purge_static(&mut self, ctx: &PrefabContext<'_, P>) -> Vec<P::Item> {
        let Some(section_count) = self.modifiers.iter().map(|m| m.range.end).max() else {
            return vec![]
        };
        let section_count = usize::try_from(section_count).unwrap();
        let mut sections = vec![self.default_section.clone(); section_count];

        let mut checker = CheckStatic::new();
        self.modifiers.retain(|modifier| {
            let is_static = checker.is_static(modifier);

            if !is_static {
                return true;
            }
            for section in modifier.range.clone() {
                let section = usize::try_from(section).unwrap();

                // SAFETY: sections.len() == max(modifiers.range.end)
                let section = unsafe { sections.get_unchecked_mut(section) };

                // TODO(clean): Check if this is reasonable
                let ModifyKind::Modify(modifier) = &modifier.kind else {
                    continue;
                };
                if let Err(err) = modifier.apply(ctx, section) {
                    self.errors.push(err);
                };
            }
            false
        });
        sections
    }

    /// The list of `Modify`s in `modifiers`.
    fn indices(&self) -> impl Iterator<Item = (Idx, &MakeModify<P>)> {
        self.modifiers
            .iter()
            .enumerate()
            .map(|(i, m)| (Idx::new(i), m))
    }
    /// The list of `Modify`s that directly depend on a root field.
    ///
    /// `field` is the property in question. A root field is the "parent style".
    fn field_f2m(&self, field: PrefabField<P>) -> impl Iterator<Item = Idx> + '_ {
        let mut parent_field_range_end = 0;

        self.indices().filter_map(move |(i, modify)| {
            let depends = modify.depends().contains(field);
            let changes = modify.changes().contains(field);

            let is_child = parent_field_range_end >= modify.range.start;
            let depends_on_parent = depends && is_child;

            if changes && parent_field_range_end < modify.range.end {
                parent_field_range_end = modify.range.end;
            }
            (depends && !depends_on_parent).then_some(i)
        })
    }

    /// The list of `Modify`s that depend on other `Modify` for their value on `field`.
    ///
    /// This is a list of parent→child tuples.
    fn field_m2m(&self, field: PrefabField<P>) -> impl Iterator<Item = (Idx, Idx)> + '_ {
        let mut parent = Vec::new();

        self.indices().filter_map(move |(i, modify)| {
            let depends = modify.depends().contains(field);
            let changes = modify.changes().contains(field);

            let mut ret = None;

            if let Some((parent_i, parent_end)) = parent.pop() {
                if parent_end >= modify.range.start {
                    parent.push((parent_i, parent_end));
                    if depends {
                        ret = Some((parent_i, i));
                    }
                }
            }
            if changes {
                parent.push((i, modify.range.end));
            }
            ret
        })
    }
    /// The list of `Modify`s that depend on other `Modify` for their value.
    ///
    /// This is a list of parent→child tuples.
    /// Unlike `field_m2m`, this is for all properties.
    fn m2m(&self) -> impl Iterator<Item = (Idx, Idx)> + '_ {
        EnumSet::ALL
            .iter()
            .flat_map(|change| self.field_m2m(change))
    }
    pub(super) fn build<const MC: usize>(
        mut self,
        ctx: &PrefabContext<'_, P>,
    ) -> (Resolver<P, MC>, Vec<P::Item>) {
        trace!("Building a RichText from {self:?}");
        let old_count = self.modifiers.len();

        let root_mask = self.root_mask();
        trace!("Root mask is {root_mask:?}");

        let sections = self.purge_static(ctx);
        let new_count = self.modifiers.len();
        trace!("Removed {} static modifiers", old_count - new_count);
        trace!("now we have {:?}", &self.modifiers);

        let m2m: BitMultiMap<_, _> = self.m2m().collect();
        trace!("m2m deps: {m2m:?}");

        let mut f2m = enum_multimap::Builder::new();
        for change in FieldsOf::<P>::ALL {
            f2m.insert(change, self.field_f2m(change));
        }
        // TODO(err): unwrap
        let f2m = f2m.build().unwrap();
        trace!("f2m: {f2m:?}");

        let mut b2m = Vec::<(Id, Idx)>::new();

        let modifiers = self.modifiers.into_iter().enumerate().map(|(i, modif)| {
            if let ModifyKind::Bound { binding, .. } = &modif.kind {
                b2m.push((*binding, Idx::new(i)));
            }
            modif.into()
        });
        let modifiers = modifiers.collect();
        trace!("modifiers: {modifiers:?}");

        b2m.sort_by_key(|(id, _)| *id);
        let b2m = KeySorted::from_sorted_iter(b2m.into_iter().assume_sorted_by_key());
        trace!("b2m: {b2m:?}");

        let with_deps = Resolver { m2m, f2m, b2m, modifiers, root_mask };
        (with_deps, sections)
    }
}
struct CheckStatic<P: Prefab> {
    parent_influence_ends: SmallVec<[u32; 4]>,
    static_parent_fields: SmallVec<[FieldsOf<P>; 4]>,
    all_static_fields: FieldsOf<P>,
}
impl<P: Prefab> CheckStatic<P> {
    fn new() -> Self {
        CheckStatic {
            parent_influence_ends: SmallVec::default(),
            static_parent_fields: SmallVec::default(),
            all_static_fields: EnumSet::EMPTY,
        }
    }
    fn ends_before_last(&self, modify: &MakeModify<P>) -> bool {
        self.parent_influence_ends
            .last()
            .map_or(true, |end| modify.range.end < *end)
    }
    fn pop_parent(&mut self) {
        if let Some(to_reset) = self.static_parent_fields.pop() {
            // SAFETY: `parent_influence_ends` and `static_parent_fields` always
            // have the same size
            unsafe { self.parent_influence_ends.pop().unwrap_unchecked() };

            self.all_static_fields -= to_reset;
        }
    }
    fn push_parent(&mut self, modify: &MakeModify<P>) {
        let changes = modify.changes();
        let old_changes = self.all_static_fields;

        let new_changes = changes - old_changes;
        let new_layer = self.ends_before_last(modify);

        if !new_changes.is_empty() {
            self.all_static_fields |= changes;

            if new_layer {
                // Keep track of fields we added to `all_static_fields` so that
                // we can remove them later
                self.static_parent_fields.push(new_changes);
                self.parent_influence_ends.push(modify.range.end);
            } else {
                // SAFETY: never fails because of `ends_before_last`
                let last_changes =
                    unsafe { self.static_parent_fields.last_mut().unwrap_unchecked() };
                *last_changes |= changes;
            }
        }
    }
    fn update_parents(&mut self, modify: &MakeModify<P>) {
        let end = modify.range.end;
        // any parent that has an end smaller than modify is actually not a parent,
        // so we pop them.
        let first_real_parent = self
            .parent_influence_ends
            .iter()
            .rposition(|p_end| *p_end <= end);
        let len = self.parent_influence_ends.len();

        let pop_count = first_real_parent.map_or(len, |i| len - i);
        for _ in 0..pop_count {
            self.pop_parent();
        }
    }
    fn is_static(&mut self, modify: &MakeModify<P>) -> bool {
        self.update_parents(modify);

        let mut depends = modify.depends().iter();
        let is_static = depends.all(|dep| self.all_static_fields.contains(dep));

        if is_static {
            self.push_parent(modify);
        }
        is_static
    }
}
