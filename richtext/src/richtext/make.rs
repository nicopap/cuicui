use std::{
    collections::{BTreeMap, BTreeSet},
    iter,
    ops::Range,
};

use bevy::{log::trace, text::TextSection};
use enumset::EnumSet;

use crate::{
    modify::{self, BindingId, Change},
    store::{BitMultiMap, EnumBitMatrix, EnumMultiMapBuilder, VarBitMatrix, VarBitMatrixBuilder},
};

use super::{Modifier, ModifyIndex as Idx, RichText};

#[derive(Debug)]
pub(super) struct Make {
    modifiers: Vec<Modifier>,
}

impl Make {
    pub(super) fn new(modifiers: Vec<Modifier>) -> Self {
        Self { modifiers }
    }
    /// The mask of static sections.
    ///
    /// If the bit is enabled, then it shouldn't be updated.
    /// This will be empty if there is no masks.
    fn binding_mask(&self, binding: BindingId) -> impl Iterator<Item = u32> + '_ {
        // TODO(bug)TODO(feat): We just assume no binding mask is needed
        // let range = self.modifiers.iter().filter(|m| m.modify.binding() == Some(binding));
        // let range = range.filter(|m| m.range.len() > 1);
        // range.flat_map(|m|)
        iter::empty()
    }
    fn all_binding_masks(&self) -> BTreeMap<BindingId, Box<[u32]>> {
        let all_bindings = self.modifiers.iter().filter_map(|m| m.modify.binding());
        all_bindings
            .map(|b| (b, self.binding_mask(b).collect::<Box<[_]>>()))
            .collect()
    }
    fn reorder_bindings(
        &self,
        ordered: impl ExactSizeIterator<Item = BindingId>,
        mut masks: BTreeMap<BindingId, Box<[u32]>>,
    ) -> VarBitMatrix {
        let mut builder = VarBitMatrixBuilder::with_capacity(ordered.len());
        for binding in ordered {
            let mask = masks.remove(&binding).unwrap();

            builder.add_row(mask.into_vec().into_iter());
        }
        builder.build()
    }
    fn change_root_mask(&self, change: Change) -> impl Iterator<Item = u32> + '_ {
        // TODO(bug): should handle things that depend on other things that themselves
        // have no dependencies (typically bindings)
        let no_deps = move |modify: &&Modifier| {
            !modify.modify.depends().contains(change) && modify.modify.changes().contains(change)
        };
        self.modifiers
            .iter()
            .filter(no_deps)
            .flat_map(|modify| modify.range.clone())
    }
    /// The mask of static sections.
    ///
    /// If the bit is enabled, then it shouldn't be updated.
    fn root_mask(&self) -> EnumBitMatrix<Change> {
        // TODO(err): unwrap
        let section_count = self.modifiers.iter().map(|m| m.range.end).max().unwrap();
        let mut root_mask = EnumBitMatrix::new(section_count as usize);

        for change in EnumSet::ALL {
            root_mask.set_row(change, self.change_root_mask(change));
        }
        root_mask
    }
    // TODO(perf): Also trim binding-dependent `Modify`
    // TODO(clean): shouldn't need `ctx`, but since it would require creating
    // references, it is impossible to create an ad-hoc empty one.
    /// Apply all `Modify` that do depend on nothing and remove them from `modifiers`.
    fn purge_static(&mut self, ctx: &modify::Context) -> Vec<TextSection> {
        let is_indy = |modify: &Modifier| modify.modify.depends() == EnumSet::EMPTY;
        let independents: BTreeSet<_> = self
            .modifiers
            .iter()
            .enumerate()
            .filter_map(|(i, modify)| is_indy(modify).then_some(i))
            .map(|i| Idx(i as u32))
            .collect();

        // TODO(err): unwrap
        let section_count = self.modifiers.iter().map(|m| m.range.end).max().unwrap();
        let mut sections =
            vec![TextSection::from_style(ctx.parent_style.clone()); section_count as usize];

        let mut i = 0;
        self.modifiers.retain(|modifier| {
            let is_dependent = !independents.contains(&Idx(i));

            i += 1;
            if is_dependent {
                return true;
            }
            for section in modifier.range.clone() {
                modifier
                    .modify
                    .apply(ctx, &mut sections[section as usize])
                    // TODO(err): unwrap
                    .unwrap();
            }
            false
        });
        sections
    }

    /// The list of `Modify`s in `modifiers`.
    fn indices(&self) -> impl Iterator<Item = (Idx, &Modifier)> {
        self.modifiers
            .iter()
            .enumerate()
            .map(|(i, m)| (Idx(i as u32), m))
    }
    /// The list of `Modify`s that directly depend on a root property.
    ///
    /// `change` is the property in question. A root property is the "parent style".
    fn change_direct_deps(&self, change: Change) -> impl Iterator<Item = Idx> + '_ {
        let mut parent_change_range_end = 0;

        self.indices().filter_map(move |(i, modify)| {
            let depends = modify.modify.depends().contains(change);
            let changes = modify.modify.changes().contains(change);

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
    fn change_modify_deps(&self, change: Change) -> impl Iterator<Item = (Idx, Idx)> + '_ {
        let mut parent = Vec::new();

        self.indices().filter_map(move |(i, modify)| {
            let depends = modify.modify.depends().contains(change);
            let changes = modify.modify.changes().contains(change);

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
    /// `Modify` that depends on a binding.
    fn binding_deps(&self) -> impl Iterator<Item = (BindingId, Range<u32>)> + '_ {
        self.modifiers
            .iter()
            .filter_map(|m| m.modify.binding().map(|b| (b, m.range.clone())))
    }
    pub(super) fn build(mut self, ctx: &modify::Context) -> (Vec<TextSection>, RichText) {
        trace!("Building a RichText from {self:?}");
        let old_count = self.modifiers.len();

        let root_mask = self.root_mask();
        trace!("Root mask is {root_mask:?}");
        let binding_masks = self.all_binding_masks();
        trace!("binding mask is {binding_masks:?}");

        let bindings: Box<[_]> = self.binding_deps().collect();

        let sections = self.purge_static(ctx);
        let new_count = self.modifiers.len();
        trace!("Removed {} static modifiers", old_count - new_count);

        let modify_deps: BitMultiMap<_, _> = self.modify_deps().collect();
        trace!("m2m deps: {modify_deps:?}");

        let mut direct_deps = EnumMultiMapBuilder::new();
        for change in EnumSet::<Change>::ALL {
            direct_deps.insert(change, self.change_direct_deps(change));
        }
        // TODO(err): unwrap
        let direct_deps = direct_deps.build().unwrap();
        trace!("c2m: {direct_deps:?}");

        trace!("bindings: {bindings:?}");
        trace!("modifiers: {:?}", &self.modifiers);

        let binding_iter = bindings.iter().map(|b| b.0);
        let binding_masks = self.reorder_bindings(binding_iter, binding_masks);
        trace!("binding masks: {binding_masks:?}");

        let with_deps = RichText {
            bindings,
            modify_deps,
            direct_deps,
            modifies: self.modifiers.into(),
            binding_masks,
            root_mask,
        };
        (sections, with_deps)
    }
}
