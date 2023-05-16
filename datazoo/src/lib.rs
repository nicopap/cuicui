#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![doc = include_str!("../README.md")]

mod bitmatrix;
mod bitmultimap;
mod bitset;
mod enumbitmatrix;
mod enummultimap;
mod jagged_array;
mod jagged_bitset;

/// Integer division rounded up.
const fn div_ceil(lhf: usize, rhs: usize) -> usize {
    (lhf + rhs - 1) / rhs
}

pub use bitmatrix::BitMatrix;
pub use bitmultimap::BitMultiMap;
pub use bitset::Bitset;
pub use enumbitmatrix::EnumBitMatrix;
pub use enummultimap::{EnumMultiMap, EnumMultiMapBuilder};
pub use jagged_array::Error as JaggedArrayError;
pub use jagged_array::JaggedArray;
pub use jagged_bitset::{JaggedBitset, JaggedBitsetBuilder};
