use std::{mem::size_of, ops::Range};

use datazoo::{jagged_bitset, Bitset, JaggedBitset};
use enumset::EnumSet;

use crate::{modify::Modify, resolve::MakeModify};

type Mask = Bitset<Vec<u32>>;

// M0 masks M1 when:
// M0 child of M1
// and ∃ c ∈ M1.C,
//   c ∈ M0.C
//   and c ∉ M0.D
fn mask_range<M: Modify>(parent: &MakeModify<M>, child: &MakeModify<M>) -> Range<u32> {
    if parent.changes() & child.changes() & !child.depends() != EnumSet::EMPTY {
        let offset = parent.range.start;
        let r = child.range.clone();
        r.start - offset..r.end - offset
    } else {
        0..0
    }
}
fn mask<M: Modify>(modifiers: &[MakeModify<M>], i: usize, m: &MakeModify<M>) -> Mask {
    let capacity = m.range.len() / size_of::<u32>();
    let mut mask = Bitset(Vec::with_capacity(capacity));

    let children = modifiers.iter().skip(i + 1).take_while(|c| m.parent_of(*c));
    mask.extend(children.flat_map(|c| mask_range(m, c)));
    mask
}

#[derive(Debug)]
pub(super) struct MaskRange {
    masks: Box<[Mask]>,
    builder: jagged_bitset::Builder,
}
impl MaskRange {
    pub(super) fn new<M: Modify>(modifiers: &[MakeModify<M>]) -> Self {
        let masks = modifiers.iter().enumerate();
        let masks = masks.map(|(i, m)| mask(modifiers, i, m)).collect();
        Self {
            masks,
            builder: jagged_bitset::Builder::with_capacity(modifiers.len()),
        }
    }
    pub(super) fn add_index(&mut self, i: usize) {
        self.builder.add_row(&self.masks[i])
    }

    pub(super) fn build(self) -> JaggedBitset {
        self.builder.build()
    }
}
