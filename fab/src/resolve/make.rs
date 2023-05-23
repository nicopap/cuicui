//! Create a dependency tree

use std::fmt;
use std::ops::Range;

use datazoo::{enum_multimap, sorted, BitMultiMap, EnumBitMatrix};
use enumset::EnumSet;
use log::trace;
use smallvec::SmallVec;

use crate::binding::Id;
use crate::prefab::{FieldsOf, Modify, Prefab, PrefabContext, PrefabField};

use super::{MakeModify, ModifyIndex as Idx, ModifyKind, Resolver};

pub(super) struct Make<'a, P: Prefab> {
    default_section: &'a P::Item,
    modifiers: Vec<super::Modifier<P>>,
    bindings: sorted::ByKeyBox<Id, Range<u32>>,
    errors: Vec<anyhow::Error>,
}
impl<P: Prefab> fmt::Debug for Make<'_, P>
where
    PrefabField<P>: fmt::Debug,
    P::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Make")
            .field("default_section", &self.default_section)
            .field("modifiers", &self.modifiers)
            .field("bindings", &self.bindings)
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
    /// - All [`Modify::changes`] of `make_modifiers` **must** be a subset of [`Modify::depends`].
    /// - [`Modify::depends`] may have exactly 1 or 0 components.
    pub(super) fn new(make_modifiers: Vec<MakeModify<P>>, default_section: &'a P::Item) -> Self {
        let mut modifiers = Vec::with_capacity(make_modifiers.len());
        let mut bindings = Vec::with_capacity(make_modifiers.len());

        for MakeModify { kind, range } in make_modifiers.into_iter() {
            match kind {
                ModifyKind::Bound(binding) => bindings.push((binding, range)),
                ModifyKind::Modify(modify) => {
                    modifiers.push(super::Modifier { inner: modify, range })
                }
            }
        }
        Self {
            modifiers,
            bindings: bindings.into(),
            default_section,
            errors: Vec::new(),
        }
    }
    fn change_root_mask(&self, change: PrefabField<P>) -> impl Iterator<Item = u32> + '_ {
        // TODO(bug): should handle things that depend on other things that themselves
        // have no dependencies (typically bindings)
        let no_deps = move |modify: &&super::Modifier<P>| {
            !modify.inner.depends().contains(change) && modify.inner.changes().contains(change)
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

        for change in EnumSet::ALL {
            root_mask.set_row(change, self.change_root_mask(change));
        }
        root_mask
    }
    /// Apply all static `Modify` and remove them from `modifiers`.
    ///
    /// A `Modify` is static when:
    /// - Either depends on nothing or depends on the
    ///    output of a static modifier.  
    /// - For all its items of influences, the components it changes are changed
    ///    by a static modifier child of itself. (TODO(pref))
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

                if let Err(err) = modifier.inner.apply(ctx, section) {
                    self.errors.push(err);
                };
            }
            false
        });
        sections
    }

    /// The list of `Modify`s in `modifiers`.
    fn indices(&self) -> impl Iterator<Item = (Idx, &super::Modifier<P>)> {
        self.modifiers
            .iter()
            .enumerate()
            .map(|(i, m)| (Idx::new(i), m))
    }
    /// The list of `Modify`s that directly depend on a root property.
    ///
    /// `change` is the property in question. A root property is the "parent style".
    fn change_direct_deps(&self, change: PrefabField<P>) -> impl Iterator<Item = Idx> + '_ {
        let mut parent_change_range_end = 0;

        self.indices().filter_map(move |(i, modify)| {
            let depends = modify.inner.depends().contains(change);
            let changes = modify.inner.changes().contains(change);

            let is_child = parent_change_range_end >= modify.range.start;
            let depends_on_parent = depends && is_child;

            if changes && parent_change_range_end < modify.range.end {
                parent_change_range_end = modify.range.end;
            }
            (depends && !depends_on_parent).then_some(i)
        })
    }

    /// The list of `Modify`s that depend on other `Modify` for their value on `change` property.
    ///
    /// This is a list of parent→child tuples.
    fn change_modify_deps(&self, change: PrefabField<P>) -> impl Iterator<Item = (Idx, Idx)> + '_ {
        let mut parent = Vec::new();

        self.indices().filter_map(move |(i, modify)| {
            let depends = modify.inner.depends().contains(change);
            let changes = modify.inner.changes().contains(change);

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
    /// Unlike `modify_deps_change`, this is for all properties.
    fn modify_deps(&self) -> impl Iterator<Item = (Idx, Idx)> + '_ {
        EnumSet::ALL
            .iter()
            .flat_map(|change| self.change_modify_deps(change))
    }
    pub(super) fn build<const MC: usize>(
        mut self,
        ctx: &PrefabContext<'_, P>,
    ) -> (Resolver<P, MC>, Vec<P::Item>) {
        trace!("Building a RichText from {self:?}");
        let old_count = self.modifiers.len();

        let root_mask = self.root_mask();
        trace!("Root mask is {root_mask:?}");
        // let binding_masks = self.all_binding_masks();
        // trace!("binding mask is {binding_masks:?}");

        let sections = self.purge_static(ctx);
        let new_count = self.modifiers.len();
        trace!("Removed {} static modifiers", old_count - new_count);

        let modify_deps: BitMultiMap<_, _> = self.modify_deps().collect();
        trace!("m2m deps: {modify_deps:?}");

        let mut direct_deps = enum_multimap::Builder::new();
        for change in FieldsOf::<P>::ALL {
            direct_deps.insert(change, self.change_direct_deps(change));
        }
        // TODO(err): unwrap
        let direct_deps = direct_deps.build().unwrap();
        trace!("c2m: {direct_deps:?}");

        trace!("bindings: {:?}", &self.bindings);
        trace!("modifiers: {:?}", &self.modifiers);

        let with_deps = Resolver {
            bindings: self.bindings,
            modify_deps,
            direct_deps,
            modifiers: self.modifiers.into(),
            root_mask,
        };
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
    fn ends_before_last(&self, modify: &super::Modifier<P>) -> bool {
        self.parent_influence_ends
            .last()
            .map_or(true, |end| modify.range.end < *end)
    }
    fn pop_parent(&mut self) {
        if let Some(to_reset) = self.static_parent_fields.pop() {
            // unwrap: `parent_influence_ends` and `static_parent_fields` always
            // have the same size
            self.parent_influence_ends.pop().unwrap();

            self.all_static_fields -= to_reset;
        }
    }
    fn push_parent(&mut self, modify: &super::Modifier<P>) {
        let changes = modify.inner.changes();
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
                // unwrap: never fails because of `ends_before_last`
                let last_changes = self.static_parent_fields.last_mut().unwrap();
                *last_changes |= changes;
            }
        }
    }
    fn update_parents(&mut self, modify: &super::Modifier<P>) {
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
    fn is_static(&mut self, modify: &super::Modifier<P>) -> bool {
        self.update_parents(modify);

        let mut depends = modify.inner.depends().iter();
        let is_static = depends.all(|dep| self.all_static_fields.contains(dep));

        if is_static {
            self.push_parent(modify);
        }
        is_static
    }
}
