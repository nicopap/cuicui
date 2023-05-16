use std::{marker::PhantomData, mem, ops::Range};

use enumset::EnumSetType;

use crate::{div_ceil, Bitset};

// TODO(clean): Manual impl of Debug using braile to show internal state.
// TODO(perf): inspect asm to see if Box's metadata store width rather than
// total len.
/// A bitset similar to [`BitMatrix`][super::BitMatrix],
/// but with a fixed column and row count, one row per `T` variant.
#[derive(Debug)]
pub struct EnumBitMatrix<T: EnumSetType>(Bitset<Box<[u32]>>, PhantomData<T>);

impl<T: EnumSetType> EnumBitMatrix<T> {
    /// Create a new [`EnumBitMatrix`].
    ///
    /// # Panics
    ///
    /// When `width * #Variant(T) > u32::MAX`.
    #[must_use]
    pub fn new(width: u32) -> Self {
        let len = width.checked_mul(T::BIT_WIDTH).unwrap() as usize;
        let data = vec![0; div_ceil(len, mem::size_of::<u32>())];
        Self(Bitset(data.into_boxed_slice()), PhantomData)
    }
    /// Enable bits from `iter` for given `change` row.
    ///
    /// Note that items of `iter` not within [`bit_width`](Self::bit_width) are ignored,
    /// and already enabled bits stay enabled.
    #[allow(clippy::missing_panics_doc)] // False positive, see inline comment
    pub fn set_row(&mut self, change: T, iter: impl Iterator<Item = u32>) {
        let row = change.enum_into_u32();
        // unwrap: this will never fails, as when constructing with `new`, we
        // verify that `len` is within bound of u32.
        let width = self.bit_width();

        let start = row * width;

        for to_set in iter.filter(|i| *i < width).map(|i| i + start) {
            // unwrap: to_set is always within range, as we `*i < width`
            self.0.enable_bit(to_set as usize).unwrap();
        }
    }
    /// The width in bits of individual rows of this [`EnumBitMatrix`].
    pub const fn bit_width(&self) -> u32 {
        self.0 .0.len() as u32 / T::BIT_WIDTH
    }
    /// Iterate over enabled bits in `change` row, limited to provided `range`.
    ///
    /// The iterator is ordered ascending, no duplicates.
    /// If the range doesn't fit within [`0..bit_width`](Self::bit_width),
    /// it will be truncated to fit within that range.
    pub fn row(&self, change: T, mut range: Range<u32>) -> impl Iterator<Item = u32> + '_ {
        let row = change.enum_into_u32();
        let width = self.bit_width();

        range.end = range.end.min(range.start + width);
        range.start = range.start.min(range.end);

        let start = row * width;

        let subrange_start = (start + range.start) as usize;
        let subrange_end = (start + range.end) as usize;

        self.0
            .ones_in_range(subrange_start..subrange_end)
            .map(move |i| i - start)
    }
}
