//! A bit matrix similar to [`BitMatrix`](super::BitMatrix),
//! but with columns of variable length like [`JaggedArray`](super::JaggedArray).

use std::{fmt, hint, iter, mem};

use sorted_iter::{assume::AssumeSortedByItemExt, SortedIterator};

use crate::{div_ceil, Bitset};

/// A bit matrix similar to [`BitMatrix`](super::BitMatrix),
/// but with columns of variable length like [`JaggedArray`](super::JaggedArray).
///
/// Use [`jagged_bitset::Builder`](`Builder`) to create a [`JaggedBitset`].
///
/// # Example
///
/// ```rust
/// use cuicui_datazoo::jagged_bitset;
///
/// let jagged = jagged_bitset::Builder::with_capacity(7)
///     .with_row([0, 2, 4, 8])
///     .with_row([63, 12, 2, 3])
///     .with_row([1, 3, 5, 7, 9, 11])
///     .with_row([])
///     .with_row([])
///     .with_row([])
///     .with_row([1, 3])
///     .build();
///
/// let row_1: Vec<_> = jagged.row(1).collect();
/// assert_eq!(&row_1, &[2, 3, 12, 63]);
///
/// let row_3: Vec<_> = jagged.row(3).collect();
/// assert_eq!(&row_3, &[]);
///
/// let row_6: Vec<_> = jagged.row(6).collect();
/// assert_eq!(&row_6, &[1, 3]);
/// ```
#[derive(Debug, Clone)]
pub struct JaggedBitset {
    ends: Box<[u32]>,
    bits: Bitset<Box<[u32]>>,
}
impl JaggedBitset {
    /// True if bits at column `x` and row `y` is enabled. False if not, or
    /// if `(x, y)` is not within the array.
    #[inline]
    pub fn bit(&self, x: usize, y: usize) -> bool {
        if y >= self.height() {
            return false;
        }
        let start = y.checked_sub(1).map_or(0, |i| self.ends[i]) as usize;
        let end = self.ends[y] as usize;

        if x >= end - start {
            return false;
        }
        self.bits.bit(start + x)
    }
    /// Return the width of the longest row.
    ///
    /// `0` if `height == 0`.
    #[inline]
    pub fn max_width(&self) -> u32 {
        let max = (0..self.height()).map(|i| self.width(i)).max();
        max.unwrap_or(0)
    }
    /// Return how many rows this jagged bitset has.
    #[inline]
    pub const fn height(&self) -> usize {
        self.ends.len()
    }
    /// Return the column count of `index` row.
    ///
    /// # Panics
    /// If `index` is greater or equal to the [`height`](Self::height).
    #[inline]
    pub fn width(&self, index: usize) -> u32 {
        self.get_width(index).unwrap()
    }
    /// Return the column count of `index` row.
    /// `None` if `index` is greater or equal to the [`height`](Self::height).
    #[inline]
    pub fn get_width(&self, index: usize) -> Option<u32> {
        if index >= self.height() {
            return None;
        }
        let start = index.checked_sub(1).map_or(0, |i| self.ends[i]);
        let end = self.ends[index];

        Some(end - start)
    }
    /// Iterate over all enabled bits in given `index` row.
    ///
    /// # Panics
    /// If `index` is greater or equal to the [`height`](Self::height).
    pub fn row(&self, index: usize) -> impl SortedIterator<Item = u32> + '_ {
        assert!(index < self.height());

        // SAFETY: we just checked index < self.ends.len()
        unsafe { self.row_unchecked(index) }
    }
    /// Iterate over all enabled bits in given `index` row.
    ///
    /// # Safety
    /// `index` **must be** lower than the row count.
    pub unsafe fn row_unchecked(&self, index: usize) -> impl SortedIterator<Item = u32> + '_ {
        if index >= self.height() {
            // SAFETY: upheld by function invariants
            unsafe { hint::unreachable_unchecked() }
            // This allows skipping bound checks on self.ends[i]
        }
        let start = index.checked_sub(1).map_or(0, |i| self.ends[i]);
        let end = self.ends[index];

        let range = start as usize..end as usize;
        let bits = self.bits.ones_in_range(range).map(move |i| i - start);
        let bits = bits.assume_sorted_by_item();

        let is_not_empty = start != end;
        is_not_empty.then_some(bits).into_iter().flatten()
    }

    /// Like [`JaggedBitset::braille_display`], but with rows and columns
    /// transposed (ie: rotated 90º clockwise and mirrored).
    ///
    /// # Example
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use cuicui_datazoo::jagged_bitset;
    ///
    /// let jagged = jagged_bitset::Builder::with_capacity(10)
    ///     .with_row([                     7, 8, 9, 10, 11, 12, 13                ])
    ///     .with_row([0,                         9, 10, 11, 12, 13, 14, 15, 16, 17])
    ///     .with_row([0, 1, 2,    4, 5, 6,                      13, 14            ])
    ///     .with_row([            4, 5,    7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17])
    ///     .with_row([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17])
    ///     .with_row([0, 1,    3, 4,       7, 8, 9, 10, 11,             15, 16, 17])
    ///     .with_row([0,    2,    4,    6,    8,    10,     12,     14,     16, 17])
    ///     .with_row([   1,    3,    5,    7,    9,     11,     13,     15, 16    ])
    ///     .with_row([                                                            ])
    ///     .with_row([                                  11,                       ])
    ///     .build();
    /// let shown = jagged.braille_trans_display().to_string();
    /// let expected = "🭴⠈⠇⣟⢕⠀🭰\n🭴⡀⢟⣏⢕⠀🭰\n🭴⣷⢸⣿⢕⢀🭰\n🭴⢻⢾⣇⢕⠀🭰\n🭴⠘⠘⠛⠋⠀🭰";
    /// assert_eq!(expected, &shown);
    /// ```
    pub const fn braille_trans_display(&self) -> BrailleTransposedDisplay {
        BrailleTransposedDisplay { bitset: self }
    }
    /// Return a struct that, when printed with [`fmt::Display`] or [`fmt::Debug`],
    /// displays the jagged bitset using unicode braille characters([wikipedia]).
    ///
    /// # Example
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use cuicui_datazoo::jagged_bitset;
    ///
    /// let jagged = jagged_bitset::Builder::with_capacity(10)
    ///     .with_row([                     7, 8, 9, 10, 11, 12, 13                ])
    ///     .with_row([0,                         9, 10, 11, 12, 13, 14, 15, 16, 17])
    ///     .with_row([0, 1, 2,    4, 5, 6,                      13, 14            ])
    ///     .with_row([            4, 5,    7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17])
    ///     .build();
    /// let shown = jagged.braille_display().to_string();
    /// let expected = "🭴⠦⠄⣤⢌⣙⣛⣻⣖⣒🭰";
    /// assert_eq!(expected, &shown);
    /// ```
    ///
    /// [wikipedia]: https://en.wikipedia.org/wiki/Braille_Patterns
    pub const fn braille_display(&self) -> BrailleDisplay {
        BrailleDisplay { bitset: self }
    }
}
/// Helps create [`JaggedBitset`] with [`Builder::build`].
///
/// [`JaggedBitset`] is immutable with a fixed capacity, so it is necessary
/// to pass through a builder ot create one.
#[derive(Debug, Clone, Default)]
pub struct Builder {
    ends: Vec<u32>,
    bits: Bitset<Vec<u32>>,
}
impl Builder {
    /// Initialize a [`Builder`].
    pub fn new() -> Self {
        Self::default()
    }
    /// Initialize a [`Builder`] with capacity rows.
    pub fn with_capacity(cap: usize) -> Self {
        Builder {
            ends: Vec::with_capacity(cap),
            bits: Bitset(Vec::new()),
        }
    }
    /// Create the immutable [`JaggedBitset`], consuming this constructor.
    pub fn build(self) -> JaggedBitset {
        JaggedBitset {
            ends: self.ends.into_boxed_slice(),
            bits: Bitset(self.bits.0.into_boxed_slice()),
        }
    }
    /// Add a single row to this [`Builder`], returning it.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cuicui_datazoo::{jagged_bitset, JaggedBitset};
    ///
    /// let jagged: JaggedBitset = jagged_bitset::Builder::with_capacity(7)
    ///     .with_row([0, 2, 4, 8])
    ///     .with_row([63, 12, 2, 3])
    ///     .with_row([1, 3, 5, 7, 9, 11])
    ///     .with_row([])
    ///     .with_row([])
    ///     .with_row([])
    ///     .with_row([1, 3])
    ///     .build();
    /// ```
    pub fn with_row(mut self, row: impl IntoIterator<Item = u32>) -> Self {
        self.add_row(row);
        self
    }
    /// Add a single row to this [`Builder`],
    /// each item of the iterator is a bit to enable in this row.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cuicui_datazoo::{jagged_bitset, JaggedBitset};
    ///
    /// let mut build = jagged_bitset::Builder::with_capacity(7);
    /// build.add_row([0, 2, 4, 8]);
    /// build.add_row([63, 12, 2, 3]);
    /// build.add_row([1, 3, 5, 7, 9, 11]);
    /// build.add_row([]);
    /// build.add_row([]);
    /// build.add_row([]);
    /// build.add_row([1, 3]);
    /// let jagged: JaggedBitset = build.build();
    /// ```
    pub fn add_row(&mut self, row: impl IntoIterator<Item = u32>) {
        let start = self.ends.last().map_or(0, |i| *i);

        let mut row_len = 0;
        for bit in row {
            let bit_in_array = (bit + start) as usize;

            if self.bits.bit_len() <= bit_in_array {
                let extra_blocks = (bit_in_array - self.bits.bit_len()) / mem::size_of::<u32>() + 1;
                self.bits.0.extend(iter::repeat(0).take(extra_blocks));
            }
            self.bits.enable_bit(bit_in_array);
            row_len = row_len.max(bit + 1);
        }
        self.ends.push(start + row_len);
    }
}

fn display_braille(
    f: &mut fmt::Formatter,
    height: usize,
    width: usize,
    get_bit: impl Fn(usize, usize) -> u32,
) -> fmt::Result {
    if width == 0 {
        write!(f, "\u{1fb74}\u{1fb70}")?;
    }
    for y in 0..div_ceil(height, 4) {
        if y != 0 {
            writeln!(f)?;
        }
        write!(f, "\u{1fb74}")?;
        for x in 0..div_ceil(width, 2) {
            let get_bit = |offset_x, offset_y| get_bit(x * 2 + offset_x, y * 4 + offset_y);
            let offset = get_bit(0, 0)
                | get_bit(1, 0) << 3
                | get_bit(0, 1) << 1
                | get_bit(1, 1) << 4
                | get_bit(0, 2) << 2
                | get_bit(1, 2) << 5
                | get_bit(0, 3) << 6
                | get_bit(1, 3) << 7;
            let character = char::from_u32(0x2800 + offset).unwrap();
            write!(f, "{character}")?;
        }
        write!(f, "\u{1fb70}")?;
    }
    Ok(())
}
/// Nice printing for [`JaggedBitset`], see [`JaggedBitset::braille_trans_display`] for details.
#[derive(Clone, Copy)]
pub struct BrailleTransposedDisplay<'a> {
    bitset: &'a JaggedBitset,
}
impl<'a> fmt::Debug for BrailleTransposedDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl<'a> fmt::Display for BrailleTransposedDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Note that since this is transposed, height/width are swapped
        let height = self.bitset.max_width() as usize;
        let width = self.bitset.height();
        display_braille(f, height, width, |x, y| self.bitset.bit(y, x) as u32)
    }
}
/// Nice printing for [`JaggedBitset`], see [`JaggedBitset::braille_display`] for details.
#[derive(Clone, Copy)]
pub struct BrailleDisplay<'a> {
    bitset: &'a JaggedBitset,
}
impl<'a> fmt::Debug for BrailleDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl<'a> fmt::Display for BrailleDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = self.bitset.max_width() as usize;
        let height = self.bitset.height();
        display_braille(f, height, width, |x, y| self.bitset.bit(x, y) as u32)
    }
}
