#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![doc = include_str!("../README.md")]

pub mod bitmatrix;
pub mod bitmultimap;
pub mod bitset;
pub mod enum_bitmatrix;
pub mod enum_multimap;
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

pub use bitmatrix::BitMatrix;
pub use bitmultimap::BitMultimap;
pub use bitset::Bitset;
pub use enum_bitmatrix::EnumBitMatrix;
pub use enum_multimap::EnumMultimap;
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
