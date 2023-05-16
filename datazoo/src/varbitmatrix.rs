use std::{iter, mem};

use super::bitset::BitSetExtensions;

/// A bit matrix similar to [`BitMatrix`](super::BitMatrix),
/// but with columns of variable length like [`VarMatrix`](super::VarMatrix).
#[derive(Debug, Clone)]
pub struct VarBitMatrix {
    ends: Box<[u32]>,
    bits: Box<[u32]>,
}
impl VarBitMatrix {
    /// Iterate over all enabled bits in given `index` row.
    ///
    /// Values are returned in unique ascending order always.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cuicui_datazoo::VarBitMatrixBuilder;
    ///
    /// let mut build = VarBitMatrixBuilder::new();
    /// build.add_row([0_u32, 2, 4, 8].into_iter());
    /// build.add_row([1_u32, 3, 5, 9].into_iter());
    /// build.add_row([0_u32, 2, 4, 8].into_iter());
    /// build.add_row([1_u32, 3, 5, 9].into_iter());
    /// let jagged = build.build();
    ///
    /// let row_2: Vec<_> = jagged.row(2).collect();
    /// assert_eq!(&row_2, &[0, 2, 4, 8]);
    ///
    /// let row_3: Vec<_> = jagged.row(3).collect();
    /// assert_eq!(&row_3, &[1, 3, 5, 9]);
    /// ```
    pub fn row(&self, index: usize) -> impl Iterator<Item = u32> + '_ {
        assert!(index < self.ends.len());

        let start = index.checked_sub(1).map_or(0, |i| self.ends[i]);
        let end = self.ends[index];

        let range = start as usize..end as usize;
        let bits = self.bits.ones_in_range(range).map(move |i| i - start);

        let is_not_empty = start != end;
        is_not_empty.then_some(bits).into_iter().flatten()
    }
}
/// Helps create [`VarBitMatrix`] with [`VarBitMatrixBuilder::build`].
///
/// [`VarBitMatrix`] is immutable with a fixed capacity, so it is necessary
/// to pass through a builder ot create one.
#[derive(Debug, Clone, Default)]
pub struct VarBitMatrixBuilder {
    ends: Vec<u32>,
    bits: Vec<u32>,
}
impl VarBitMatrixBuilder {
    /// Initialize a [`VarBitMatrixBuilder`].
    pub fn new() -> Self {
        Self::default()
    }
    /// Initialize a [`VarBitMatrixBuilder`] with capacity rows.
    pub fn with_capacity(cap: usize) -> Self {
        VarBitMatrixBuilder { ends: Vec::with_capacity(cap), bits: Vec::new() }
    }
    /// Create the immutable [`VarBitMatrix`], consuming this constructor.
    pub fn build(self) -> VarBitMatrix {
        VarBitMatrix {
            ends: self.ends.into_boxed_slice(),
            bits: self.bits.into_boxed_slice(),
        }
    }
    /// Add a single row to this [`VarBitMatrixBuilder`],
    /// each item of the iterator is a bit to enable in this row.
    pub fn add_row(&mut self, row: impl Iterator<Item = u32>) {
        let end = self.ends.last().map_or(0, |i| *i);

        let mut this_row_length = 0;
        for cell in row {
            let cell_u = (cell + end) as usize;
            if self.bits.bit_len() <= cell_u {
                let to_add = (cell_u - self.bits.bit_len()) / mem::size_of::<f32>() + 1;
                self.bits.extend(iter::repeat(0).take(to_add));
            }
            self.bits.enable_bit(cell_u);
            this_row_length = this_row_length.max(cell);
        }
        self.ends.push(end + this_row_length + 1);
    }
}
