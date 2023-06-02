//! Create a dependency tree

mod is_static;
mod mask_range;

use std::{fmt, mem::size_of};

use datazoo::{enum_multimap, sorted::KeySorted, AssumeSortedByKeyExt, BitMultimap, JaggedBitset};
use enumset::EnumSet;
use log::{error, trace};

use crate::binding::Id;
use crate::prefab::{FieldsOf, Modify, Prefab, PrefabContext, PrefabField};

use super::{MakeModify, ModifyIndex as Idx, ModifyKind, Resolver};
use is_static::CheckStatic;
use mask_range::MaskRange;

pub(super) struct Make<'a, P: Prefab> {
    default_section: &'a P::Item,
    modifiers: Vec<super::MakeModify<P>>,
    errors: Vec<anyhow::Error>,
}
// Manual `impl` because we don't want `Make: Debug where P: Debug`, only
// `Make: Debug where P::Item: Debug, PrefabField<P>: Debug`
impl<P: Prefab> fmt::Debug for Make<'_, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Make")
            .field("default_section", &self.default_section)
            .field("modifiers", &self.modifiers)
            .field("errors", &self.errors)
            .finish()
    }
}

impl<'a, P: Prefab> Make<'a, P> {
    /// Initialize a [`Make`] to create a [`Resolver`] using [`Make::build`].
    ///
    /// # Limitations
    ///
    /// - All [`Modify::changes`] of `modifiers` **must** be a subset of [`Modify::depends`].
    /// - [`Modify::depends`] may have exactly 1 or 0 components.
    pub(super) fn new(modifiers: Vec<MakeModify<P>>, default_section: &'a P::Item) -> Self {
        Self { modifiers, default_section, errors: Vec::new() }
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
    fn purge_static(&mut self, ctx: &PrefabContext<'_, P>) -> (Vec<P::Item>, JaggedBitset) {
        assert!(size_of::<usize>() >= size_of::<u32>());

        let Some(section_count) = self.modifiers.iter().map(|m| m.range.end).max() else {
            return (vec![], JaggedBitset::default())
        };
        let mut sections = vec![self.default_section.clone(); section_count as usize];

        let mut checker = CheckStatic::new();
        let mut masker = MaskRange::new(&self.modifiers);
        let mut i = 0;

        self.modifiers.retain(|modifier| {
            let current_index = i;
            i += 1;

            let is_static = checker.is_static(modifier);

            if !is_static {
                masker.add_index(current_index);
                return true;
            }
            for section in modifier.range.clone() {
                // SAFETY: sections.len() == max(modifiers.range.end)
                let section = unsafe { sections.get_unchecked_mut(section as usize) };
                let ModifyKind::Modify(modifier) = &modifier.kind else { continue; };

                if let Err(err) = modifier.apply(ctx, section) {
                    self.errors.push(err);
                };
            }
            false
        });
        (sections, masker.build())
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

            while let Some((parent_i, parent_end)) = parent.pop() {
                if parent_end > modify.range.start {
                    parent.push((parent_i, parent_end));
                    if depends {
                        ret = Some((parent_i, i));
                    }
                    trace!("{i:?} has parent {parent_i:?} (depends: {depends})");
                    break;
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
        trace!("Building a RichText from modifiers:");
        for modi in &self.modifiers {
            trace!("\t{modi:?}");
        }
        let old_count = self.modifiers.len();

        let (sections, masks) = self.purge_static(ctx);
        let new_count = self.modifiers.len();

        trace!("masks are {}", masks.braille_trans_display());
        trace!("Removed {} static modifiers", old_count - new_count);
        trace!("now we have:");
        for (i, modi) in self.modifiers.iter().enumerate() {
            trace!("\t<M{i}>: {modi:?}");
        }

        let m2m: BitMultimap<_, _> = self.m2m().collect();
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

        b2m.sort_by_key(|d| d.0);
        let b2m = KeySorted::from_sorted_iter(b2m.into_iter().assume_sorted_by_key());
        trace!("b2m: {b2m:?}");

        if !self.errors.is_empty() {
            error!("Errors while creating resolver:");
            for err in &self.errors {
                error!("\t{err}");
            }
        }
        let with_deps = Resolver { m2m, f2m, b2m, modifiers, masks };
        (with_deps, sections)
    }
}
