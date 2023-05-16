#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![doc = include_str!("../README.md")]

mod bitmatrix;
mod bitmultimap;
mod bitset;
mod enumbitmatrix;
mod enummultimap;
mod varbitmatrix;
mod varmatrix;

/// Integer division rounded up.
const fn div_ceil(lhf: usize, rhs: usize) -> usize {
    (lhf + rhs - 1) / rhs
}

pub use bitmatrix::BitMatrix;
pub use bitmultimap::BitMultiMap;
pub use bitset::Bitset;
pub use enumbitmatrix::EnumBitMatrix;
pub use enummultimap::{EnumMultiMap, EnumMultiMapBuilder};
pub use varbitmatrix::{VarBitMatrix, VarBitMatrixBuilder};
pub use varmatrix::Error as VarMatrixError;
pub use varmatrix::VarMatrix;
