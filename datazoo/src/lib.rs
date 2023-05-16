#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![doc = include_str!("../README.md")]

mod bitmatrix;
mod bitmultimap;
mod bitset;
pub mod enum_multimap;
mod enumbitmatrix;
pub mod jagged_array;
pub mod jagged_bitset;
pub mod sorted;

/// Integer division rounded up.
const fn div_ceil(lhf: usize, rhs: usize) -> usize {
    (lhf + rhs - 1) / rhs
}

pub use bitmatrix::BitMatrix;
pub use bitmultimap::BitMultiMap;
pub use bitset::Bitset;
pub use enum_multimap::EnumMultiMap;
pub use enumbitmatrix::EnumBitMatrix;
pub use jagged_array::JaggedArray;
pub use jagged_bitset::JaggedBitset;
pub use sorted_iter::assume::{AssumeSortedByItemExt, AssumeSortedByKeyExt};
pub use sorted_iter::{SortedIterator, SortedPairIterator};
