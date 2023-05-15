use std::{marker::PhantomData, mem, ops::Range};

use enumset::EnumSetType;

use super::div_ceil;
use crate::store::BitSetExtensions;

// TODO(clean): Manual impl of Debug using braile to show internal state.
// TODO(perf): inspect asm to see if Box's metadata store width rather than
// total len.
/// A bitset similar to [`BitMatrix`][super::BitMatrix],
/// but with a fixed column and row count, one row per `T` variant.
#[derive(Debug)]
pub struct EnumBitMatrix<T: EnumSetType>(Box<[u32]>, PhantomData<T>);

impl<T: EnumSetType> EnumBitMatrix<T> {
    pub fn new(width: usize) -> Self {
        let len = width * T::BIT_WIDTH as usize;
        let data = vec![0; div_ceil(len, mem::size_of::<u32>())];
        Self(data.into_boxed_slice(), PhantomData)
    }
    pub fn set_row(&mut self, change: T, iter: impl Iterator<Item = u32>) {
        let row = change.enum_into_u32();
        let width = self.0.len() as u32 / T::BIT_WIDTH;

        let start = row * width;

        for to_set in iter.filter(|i| *i < width).map(|i| i + start) {
            // unwrap: to_set is always within range, as we `*i < width`
            self.0.enable_bit(to_set as usize).unwrap();
        }
    }
    pub fn row(&self, change: T, range: Range<u32>) -> impl Iterator<Item = u32> + '_ {
        let row = change.enum_into_u32();
        let width = self.0.len() as u32 / T::BIT_WIDTH;
        assert!(range.end - range.start <= width);

        let start = row * width;

        let subrange_start = (start + range.start) as usize;
        let subrange_end = (start + range.end) as usize;

        self.0
            .ones_in_range(subrange_start..subrange_end)
            .map(move |i| i - start)
    }
}
