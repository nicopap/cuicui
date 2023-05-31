use enumset::EnumSet;
use smallvec::SmallVec;

use super::{MakeModify, ModifyKind};
use crate::prefab::{FieldsOf, Prefab};

pub(super) struct CheckStatic<P: Prefab> {
    parents_range_end: SmallVec<[u32; 4]>,
    static_parent_fields: SmallVec<[FieldsOf<P>; 4]>,
    all_static_fields: FieldsOf<P>,
}
impl<P: Prefab> CheckStatic<P> {
    pub(super) fn new() -> Self {
        CheckStatic {
            parents_range_end: SmallVec::default(),
            static_parent_fields: SmallVec::default(),
            all_static_fields: EnumSet::EMPTY,
        }
    }
    fn ends_before_last(&self, modify: &MakeModify<P>) -> bool {
        self.parents_range_end
            .last()
            .map_or(true, |end| modify.range.end < *end)
    }
    fn pop_parent(&mut self) {
        if let Some(to_reset) = self.static_parent_fields.pop() {
            // SAFETY: `parents_range_end` and `static_parent_fields` always
            // have the same size
            unsafe { self.parents_range_end.pop().unwrap_unchecked() };

            self.all_static_fields -= to_reset;
        }
    }
    fn push_parent(&mut self, modify: &MakeModify<P>) {
        let changes = modify.changes();
        let old_changes = self.all_static_fields;
        let modify_changes = changes - old_changes;

        // There is no changes, therefore nothing new to track
        if modify_changes.is_empty() {
            return;
        }
        self.all_static_fields |= changes;
        if self.ends_before_last(modify) {
            // Keep track of fields we added to `all_static_fields` so that
            // we can remove them later
            self.static_parent_fields.push(modify_changes);
            self.parents_range_end.push(modify.range.end);
        } else {
            // SAFETY: never fails because of `ends_before_last` is only false
            // when there is at least one element in static_parent_fields
            let last_changes = unsafe { self.static_parent_fields.last_mut().unwrap_unchecked() };
            *last_changes |= changes;
        }
    }
    fn update_parents(&mut self, modify: &MakeModify<P>) {
        let end = modify.range.end;
        // any parent that has an end smaller than modify is actually not a parent,
        // so we pop them.
        let first_real_parent = self
            .parents_range_end
            .iter()
            // TODO(clean): Is this right? Shouldn't it be < over <=?
            .rposition(|p_end| *p_end <= end);
        let len = self.parents_range_end.len();

        let pop_count = len - first_real_parent.unwrap_or(0);
        for _ in 0..pop_count {
            self.pop_parent();
        }
    }
    pub(super) fn is_static(&mut self, modify: &MakeModify<P>) -> bool {
        self.update_parents(modify);

        let mut depends = modify.depends().iter();
        let no_deps = depends.all(|dep| self.all_static_fields.contains(dep));
        let is_binding = matches!(&modify.kind, ModifyKind::Bound { .. });

        let is_static = no_deps && !is_binding;

        if is_static {
            self.push_parent(modify);
        }
        is_static
    }
}
