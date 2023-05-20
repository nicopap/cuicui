//! Create a dependency tree

use std::fmt;
use std::{collections::BTreeSet, ops::Range};

use bevy_log::trace;
use datazoo::{enum_multimap, sorted, BitMultiMap, EnumBitMatrix};
use enumset::EnumSet;

use crate::binding::BindingId;
use crate::prefab::{Modify, Prefab};

use super::{MakeModifier as Modifier, ModifyIndex as Idx, ModifyKind, Resolver};

#[derive(Debug)]
pub(super) struct Make<P: Prefab> {
    default_section: P,
    modifiers: Vec<super::Modifier<P>>,
    bindings: sorted::ByKeyBox<BindingId, Range<u32>>,
}

impl<P> Make<P>
where
    P: Prefab + Clone + fmt::Debug,
    P::Field: fmt::Debug,
{
    pub(super) fn new(make_modifiers: Vec<Modifier<P>>, default_section: P) -> Self {
        let mut modifiers = Vec::with_capacity(make_modifiers.len());
        let mut bindings = Vec::with_capacity(make_modifiers.len());

        for Modifier { kind, range } in make_modifiers.into_iter() {
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
        }
    }
    fn change_root_mask(&self, change: P::Field) -> impl Iterator<Item = u32> + '_ {
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
    fn root_mask(&self) -> EnumBitMatrix<P::Field> {
        // TODO(err): unwrap
        let section_count = self.modifiers.iter().map(|m| m.range.end).max().unwrap();
        let mut root_mask = EnumBitMatrix::new(section_count);

        for change in EnumSet::ALL {
            root_mask.set_row(change, self.change_root_mask(change));
        }
        root_mask
    }
    // TODO(clean): shouldn't need `ctx`, but since it would require creating
    // references, it is impossible to create an ad-hoc empty one.
    /// Apply all `Modify` that do depend on nothing and remove them from `modifiers`.
    fn purge_static(&mut self, ctx: &P::Context) -> Vec<P> {
        let is_indy = |modify: &super::Modifier<P>| modify.inner.depends() == EnumSet::EMPTY;
        let independents: BTreeSet<_> = self
            .modifiers
            .iter()
            .enumerate()
            .filter_map(|(i, modify)| is_indy(modify).then_some(i))
            .map(Idx::new)
            .collect();

        // TODO(err): unwrap
        let section_count = self.modifiers.iter().map(|m| m.range.end).max().unwrap();
        let mut sections = vec![self.default_section.clone(); section_count as usize];

        let mut i = 0;
        self.modifiers.retain(|modifier| {
            let is_dependent = !independents.contains(&Idx(i));

            i += 1;
            if is_dependent {
                return true;
            }
            for section in modifier.range.clone() {
                modifier
                    .inner
                    .apply(ctx, sections.get_mut(section as usize).unwrap())
                    // TODO(err): unwrap
                    .unwrap();
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
            .map(|(i, m)| (Idx(i as u32), m))
    }
    /// The list of `Modify`s that directly depend on a root property.
    ///
    /// `change` is the property in question. A root property is the "parent style".
    fn change_direct_deps(&self, change: P::Field) -> impl Iterator<Item = Idx> + '_ {
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
    fn change_modify_deps(&self, change: P::Field) -> impl Iterator<Item = (Idx, Idx)> + '_ {
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
    // TODO(clean): verify that I can do that independently of the change.
    /// The list of `Modify`s that depend on other `Modify` for their value.
    ///
    /// This is a list of parent→child tuples.
    /// Unlike `modify_deps_change`, this is for all properties.
    fn modify_deps(&self) -> impl Iterator<Item = (Idx, Idx)> + '_ {
        EnumSet::ALL
            .iter()
            .flat_map(|change| self.change_modify_deps(change))
    }
    pub(super) fn build<const MC: usize>(mut self, ctx: &P::Context) -> (Resolver<P, MC>, Vec<P>) {
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
        for change in EnumSet::<P::Field>::ALL {
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