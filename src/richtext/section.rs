// TODO(perf): Not require `clone_dyn` on Modify. Would require `RichText` to be
// a specialized datastructure.
//
// - A `Vec<HashMap<TypeId, (ModifierBox, usize)>>` where `usize` tells for how
//   many additional positions in the `Vec` the modifier is relevant.
//   It would require accumulating ModifierBoxes when traversing sections.
//   It would be made extra tricky if we allow modifier shadowing (you need to
//   keep track of previous value of modifiers) But currently we don't allow that
//   so the list of modifiers is flat.
// - `Vec<HashMap<TypeId, OrRef<ModifierBox>>>` where `enum OrRef { Actual(T), AtIndex(usize) }`
//   would replace the clone with adding a `OrRef::AtIndex(index_or_ref_actual)` to
//   subsequent indices where ModifierBox should be cloned.
//   When traversing, we could just sections[at_index][type_id] to access the actual
//   value.
// - `Vec<HashMap<TypeId, OrPtr>>` where `enum OrPtr { Box(ModifierBox), Ptr(*const dyn Modify) }`
//   replace the clone with a ptr copy, can implement `AsRef` for `OrPtr`, traversing
//   is equivalent to the naive `clone_dyn`-based implementation.
//   The only worry here is to make sure the `Ptr` variants are dropped at the same time
//   as the Box, (at least I think?)
// TODO(perf): should have change tracking (might require internal mutability)
// to be precise and extremely limited about what we update.
#[derive(PartialEq, Debug, Default)]
pub struct Section {
    pub(super) modifiers: super::Modifiers,
}
