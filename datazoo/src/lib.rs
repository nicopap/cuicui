#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![doc = include_str!("../README.md")]

pub mod bitmatrix;
pub mod bitmultimap;
pub mod bitset;
pub mod enum_bitmatrix;
pub mod enum_multimap;
// pub mod index_map;
pub mod index_multimap;
pub mod jagged_bitset;
pub mod jagged_const_row_array;
pub mod jagged_vec;
pub mod raw_index_map;
pub mod sorted;

/// Integer division rounded up.
const fn div_ceil(lhf: usize, rhs: usize) -> usize {
    (lhf + rhs - 1) / rhs
}
trait MostSignificantBit {
    fn most_significant_bit(&self) -> u32;
}
impl MostSignificantBit for u32 {
    fn most_significant_bit(&self) -> u32 {
        u32::BITS - self.leading_zeros()
    }
}
impl MostSignificantBit for usize {
    fn most_significant_bit(&self) -> u32 {
        usize::BITS - self.leading_zeros()
    }
}

/// Get an `usize` from `Self`.
#[rustfmt::skip]
mod index {
    pub trait Index { fn get(&self) -> usize; }
    impl Index for usize { fn get(&self) -> usize { *self } }
    impl Index for u32 { fn get(&self) -> usize { *self as usize } }
    impl Index for u64 { fn get(&self) -> usize { *self as usize } }
}

pub use bitmatrix::BitMatrix;
pub use bitmultimap::BitMultimap;
pub use bitset::Bitset;
pub use enum_bitmatrix::EnumBitMatrix;
pub use enum_multimap::EnumMultimap;
pub use index::Index;
pub use index_multimap::IndexMultimap;
pub use jagged_bitset::JaggedBitset;
pub use jagged_const_row_array::JaggedConstRowArray;
pub use jagged_vec::JaggedVec;
pub use raw_index_map::RawIndexMap;
pub use sorted_iter::assume::{AssumeSortedByItemExt, AssumeSortedByKeyExt};
pub use sorted_iter::{
    sorted_iterator::SortedByItem, sorted_pair_iterator::SortedByKey, SortedIterator,
    SortedPairIterator,
};
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msb() {
        assert_eq!(101_u32.most_significant_bit(), 7);
        assert_eq!(10_u32.most_significant_bit(), 4);
        assert_eq!(0b0000_u32.most_significant_bit(), 0);
        assert_eq!(0b0001_u32.most_significant_bit(), 1);
        assert_eq!(0b0010_u32.most_significant_bit(), 2);
        assert_eq!(0b0011_u32.most_significant_bit(), 2);
        assert_eq!(0b0100_u32.most_significant_bit(), 3);
        assert_eq!(0b0101_u32.most_significant_bit(), 3);
        assert_eq!(0b0110_u32.most_significant_bit(), 3);
        assert_eq!(0b0111_u32.most_significant_bit(), 3);
        assert_eq!(0b1000_u32.most_significant_bit(), 4);
        assert_eq!(0b1001_u32.most_significant_bit(), 4);
        assert_eq!(0b1010_u32.most_significant_bit(), 4);
        assert_eq!(0b1011_u32.most_significant_bit(), 4);
        assert_eq!(0b1100_u32.most_significant_bit(), 4);
        assert_eq!(0b1101_u32.most_significant_bit(), 4);
        assert_eq!(0b1110_u32.most_significant_bit(), 4);
        assert_eq!(0b1111_u32.most_significant_bit(), 4);
        assert_eq!(0b0100_0000_0000u32.most_significant_bit(), 11);
        assert_eq!(0b1000_0000_0000_0000u32.most_significant_bit(), 16);
        assert_eq!(0b0010_0000_0000_0000u32.most_significant_bit(), 14);
        assert_eq!(0xf000_0000u32.most_significant_bit(), 32);
        assert_eq!(0xffff_ffffu32.most_significant_bit(), 32);
        assert_eq!(0xffff_0000u32.most_significant_bit(), 32);
    }
}
