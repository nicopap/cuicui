//! A [bitset](Bitset) with fixed-size rows.

use std::{fmt, mem};

use crate::{div_ceil, Bitset};

/// A [bitset](Bitset) with fixed-size rows.
///
/// Note that only the total size is tracked in `BitMatrix` and you must provide
/// the `width` value when calling methods on `BitMatrix`.
#[derive(Debug)]
pub struct BitMatrix(Bitset<Box<[u32]>>);
impl BitMatrix {
    /// Iterate over active bits in given `column`.
    ///
    /// # Panics
    ///
    /// When `width = 0` (this would otherwise mean there is an infinite
    /// amount of columns)
    #[inline]
    pub fn active_rows_in_column(&self, width: usize, column: usize) -> Column {
        assert_ne!(width, 0);
        Column { data: &self.0 .0, width, current_cell: column }
    }
    pub fn row(&self, width: usize, row: usize) -> impl Iterator<Item = usize> + '_ {
        let start = row * width;
        let end = (row + 1) * width;

        self.0
            .ones_in_range(start..end)
            .map(move |i| (i as usize) - start)
    }
    #[inline]
    pub fn enable_bit(&mut self, width: usize, row: usize, column: usize) -> Option<()> {
        self.0.enable_bit(width * row + column)
    }
    /// Create a [`BitMatrix`] with given proportions.
    #[must_use]
    pub fn new_with_size(width: usize, height: usize) -> Self {
        let bit_size = width * height;
        let u32_size = div_ceil(bit_size, mem::size_of::<u32>());
        BitMatrix(Bitset(vec![0; u32_size].into_boxed_slice()))
    }

    /// `true` if bit at position `x, y` in matrix is enabled.
    ///
    /// `false` otherwise, included if `x, y` is outside of the matrix.
    pub fn bit(&self, x: usize, y: usize, width: usize) -> bool {
        x < width && self.0.bit(x + y * width)
    }

    /// Return a struct that, when printed with [`fmt::Display`] or [`fmt::Debug`],
    /// displays the matrix using unicode sextant characters([pdf]).
    ///
    /// [pdf]: https://unicode.org/charts/PDF/U1FB00.pdf
    pub const fn sextant_display(&self, width: usize, height: usize) -> SextantDisplay {
        SextantDisplay { matrix: self, width, height }
    }
}

/// Iterator over a single column of a [`BitMatrix`],
/// see [`BitMatrix::active_rows_in_column`] documentation for details.
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
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let upper = self.data.len().saturating_sub(self.current_cell) / self.width;
        (0, Some(upper))
    }
    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.current_cell = self.current_cell.saturating_add(n * self.width);
        self.next()
    }
}

/// Nice printing for [`BitMatrix`], see [`BitMatrix::sextant_display`] for details.
#[derive(Copy, Clone)]
pub struct SextantDisplay<'a> {
    matrix: &'a BitMatrix,
    width: usize,
    height: usize,
}
impl<'a> fmt::Debug for SextantDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl<'a> fmt::Display for SextantDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.height == 0 {
            write!(f, "\u{1fb74}\u{1fb70}")?;
        }
        for y in 0..div_ceil(self.height, 3) {
            if y != 0 {
                writeln!(f)?;
            }
            write!(f, "\u{1fb74}")?;
            for x in 0..div_ceil(self.width, 2) {
                let get_bit = |offset_x, offset_y| {
                    let (x, y) = (x * 2 + offset_x, y * 3 + offset_y);
                    self.matrix.bit(x, y, self.width) as u32
                };
                let offset = get_bit(0, 0)
                    | get_bit(1, 0) << 1
                    | get_bit(0, 1) << 2
                    | get_bit(1, 1) << 3
                    | get_bit(0, 2) << 4
                    | get_bit(1, 2) << 5;
                let character = match offset {
                    0b111111 => '\u{2588}',
                    0b000000 => ' ',
                    offset => char::from_u32(0x1fb00 + offset - 1).unwrap(),
                };
                write!(f, "{character}")?;
            }
            write!(f, "\u{1fb70}")?;
        }
        Ok(())
    }
}
