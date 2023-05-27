use core::fmt;
use std::{any, marker::PhantomData, mem, ops::Range};

use enumset::{EnumSet, EnumSetType};
use sorted_iter::{assume::AssumeSortedByItemExt, sorted_iterator::SortedByItem, SortedIterator};

use crate::{div_ceil, Bitset};

// TODO(clean): Manual impl of Debug using braile to show internal state.
/// A bitset similar to [`BitMatrix`][super::BitMatrix],
/// but with a fixed column and row count, one row per `R` variant.
#[derive(Clone, PartialEq, Eq)]
pub struct EnumBitMatrix<R: EnumSetType>(Bitset<Box<[u32]>>, PhantomData<R>);
impl<R: EnumSetType> fmt::Debug for EnumBitMatrix<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnumBitMatrix")
            .field("R", &any::type_name::<R>())
            .field("inner", &self.0)
            .finish()
    }
}

impl<R: EnumSetType> EnumBitMatrix<R> {
    /// Create a new [`EnumBitMatrix`].
    ///
    /// # Panics
    ///
    /// When `width * #Variant(T) > u32::MAX`.
    #[must_use]
    pub fn new(width: u32) -> Self {
        let len = width.checked_mul(R::BIT_WIDTH).unwrap() as usize;
        let data = vec![0; div_ceil(len, mem::size_of::<u32>())];
        Self(Bitset(data.into_boxed_slice()), PhantomData)
    }
    /// Enable bits from `iter` for given `row`.
    ///
    /// Note that items of `iter` not within [`bit_width`](Self::bit_width) are ignored,
    /// and already enabled bits stay enabled.
    #[allow(clippy::missing_panics_doc)] // False positive, see inline comment
    pub fn set_row(&mut self, row: R, iter: impl Iterator<Item = u32>) {
        let row = row.enum_into_u32();
        let width = self.bit_width();

        let start = row * width;

        for to_set in iter.filter(|i| *i < width).map(|i| i + start) {
            // unwrap: to_set is always within range, as we `*i < width`
            self.0.enable_bit(to_set as usize).unwrap();
        }
    }
    /// The width in bits of individual rows of this [`EnumBitMatrix`].
    pub const fn bit_width(&self) -> u32 {
        self.0 .0.len() as u32 / R::BIT_WIDTH
    }
    /// Iterate over enabled bits in `row`, limited to provided `range`.
    ///
    /// If the range doesn't fit within [`0..bit_width`](Self::bit_width),
    /// it will be truncated to fit within that range.
    pub fn row(&self, row: R, mut range: Range<u32>) -> impl SortedIterator<Item = u32> + '_ {
        let row = row.enum_into_u32();
        let width = self.bit_width();

        range.end = range.end.min(range.start + width);
        range.start = range.start.min(range.end);

        let start = row * width;

        let subrange_start = (start + range.start) as usize;
        let subrange_end = (start + range.end) as usize;

        self.0
            .ones_in_range(subrange_start..subrange_end)
            .map(move |i| i - start)
            .assume_sorted_by_item()
    }

    /// Iterate over enabled bits in all `rows`, limited to provided `range`.
    ///
    /// [`Rows`] is a sorted iterator.
    pub const fn rows(&self, rows: EnumSet<R>, range: Range<u32>) -> Rows<R> {
        Rows { range, rows, bitset: self }
    }
}

/// Iterator from [`EnumBitMatrix::rows`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rows<'a, R: EnumSetType> {
    range: Range<u32>,
    rows: EnumSet<R>,

    bitset: &'a EnumBitMatrix<R>,
}
impl<'a, R: EnumSetType> Iterator for Rows<'a, R> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.range.is_empty() {
            return None;
        }
        let range = self.range.clone();
        self.range.start += 1;

        self.rows
            .iter()
            .find_map(|row| self.bitset.row(row, range.clone()).next())
    }
}
impl<R: EnumSetType> SortedByItem for Rows<'_, R> {}
