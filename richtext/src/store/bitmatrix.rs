use std::mem;

use super::{bitset::BitSetExtensions, div_ceil};

pub struct Column<'a> {
    width: usize,
    current_cell: usize,
    data: &'a [u32],
}
impl Iterator for Column<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let bit = self.current_cell;
            let row = self.current_cell / self.width;
            self.current_cell += self.width;

            let block = bit / u32::BITS as usize;
            let offset = bit % u32::BITS as usize;

            let is_active = |block: u32| block & (1 << offset) != 0;
            match self.data.get(block) {
                Some(block) if is_active(*block) => return Some(row),
                Some(_) => continue,
                None => return None,
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let upper = self.data.len().saturating_sub(self.current_cell) / self.width;
        (0, Some(upper))
    }
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.current_cell = self.current_cell.saturating_add(n * self.width);
        self.next()
    }
}

/// A bitset accessible by range.
#[derive(Debug)]
pub struct BitMatrix(Box<[u32]>);
impl BitMatrix {
    pub fn active_rows_in_column(
        &self,
        width: usize,
        column: usize,
    ) -> impl Iterator<Item = usize> + '_ {
        assert_ne!(width, 0);
        Column { data: &self.0, width, current_cell: column }
    }
    pub fn row(&self, width: usize, row: usize) -> impl Iterator<Item = usize> + '_ {
        let start = row * width;
        let end = (row + 1) * width;

        self.0
            .ones_in_range(start..end)
            .map(move |i| (i as usize) - start)
    }
    pub fn enable_bit(&mut self, width: usize, row: usize, column: usize) -> Option<()> {
        self.0.enable_bit(width * row + column)
    }
    pub fn new_with_size(width: usize, height: usize) -> Self {
        let bit_size = width * height;
        let u32_size = div_ceil(bit_size, mem::size_of::<u32>());
        BitMatrix(vec![0; u32_size].into_boxed_slice())
    }
}
