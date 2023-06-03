//! An extensible (ie: can add more rows) [jagged array].
//!
//! [jagged array]: https://en.wikipedia.org/wiki/Jagged_array

use std::mem::size_of;

use thiserror::Error;

/// [`JaggedVec::new`] construction error.
#[derive(Debug, Error)]
pub enum Error {
    /// An `end` in `ends` was lower than a previous one.
    #[error(
        "Cannot build JaggedVec: `ends` represents the end of each row in `data`, \
        it should be monotonically increasing. \
        Found `end` at position {i} lower than `end` at position {}", .i - 1
    )]
    BadEnd { i: usize },
    /// An `end` in `ends` was too large.
    #[error(
        "Cannot build JaggedVec: `ends` represents the end of each row in `data`, \
        Yet, `end` at position {i} ({end}) is larger than the length of data ({len})"
    )]
    TooLongEnd { i: usize, len: u32, end: u32 },
}

/// An extensible (ie: can add more rows) [jagged array].
#[derive(Debug, PartialEq, Clone)]
pub struct JaggedVec<T> {
    ends: Vec<u32>,
    data: Vec<T>,
}
impl<T> JaggedVec<T> {
    pub fn push_row(&mut self, row: impl IntoIterator<Item = T>) {
        self.ends.push(self.data.len() as u32);
        self.data.extend(row);
    }
    /// How many cells are contained in this `JaggedVec`.
    pub fn len(&self) -> usize {
        self.data.len()
    }
    /// How many rows this `JaggedVec` has.
    pub fn height(&self) -> usize {
        self.ends.len() + 1
    }
    /// Create a [`JaggedVec`] of `ends.len() + 1` rows, values of `ends` are the
    /// end indicies (exclusive) of each row in `data`.
    ///
    /// Note that the _last index_ should be elided.
    /// The last row will be the values between the last `end` in `ends` and
    /// the total size of the `data` array.
    ///
    /// Returns `Err` if:
    ///
    /// - An `ends[i] > data.len()`
    /// - An `ends[i+1] < ends[i]`
    ///
    /// # Example
    ///
    /// ```rust
    /// use cuicui_datazoo::JaggedVec;
    ///
    /// let ends = [0, 0, 3, 4, 7, 9, 10, 10];
    /// let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 23];
    /// let jagged = JaggedVec::new(ends.to_vec(), data.to_vec()).unwrap();
    /// let iliffe = jagged.into_vecs();
    /// assert_eq!(
    ///     iliffe,
    ///     vec![
    ///         vec![],
    ///         vec![],
    ///         vec![0, 1, 2],
    ///         vec![3],
    ///         vec![4, 5, 6],
    ///         vec![7, 8],
    ///         vec![9],
    ///         vec![],
    ///         vec![11, 23],
    ///     ],
    /// );
    /// ```
    pub fn new(ends: Vec<u32>, data: Vec<T>) -> Result<Self, Error> {
        assert!(size_of::<usize>() >= size_of::<u32>());

        let mut previous_end = 0;
        let last_end = data.len() as u32;
        for (i, end) in ends.iter().enumerate() {
            if *end > last_end {
                return Err(Error::TooLongEnd { i, len: last_end, end: *end });
            }
            if *end < previous_end {
                return Err(Error::BadEnd { i });
            }
            previous_end = *end;
        }
        Ok(Self { ends, data })
    }
    /// Get slice to row at given `index`.
    ///
    /// # Panics
    ///
    /// When `index > self.height()`
    ///
    /// # Example
    ///
    /// ```rust
    /// use cuicui_datazoo::JaggedVec;
    ///
    /// let ends = [0, 0, 3, 4, 7, 9, 10, 10];
    /// let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// let jagged = JaggedVec::new(ends.to_vec(), data.to_vec()).unwrap();
    ///
    /// assert_eq!(jagged.row(4), &[4, 5, 6]);
    /// ```
    #[inline]
    pub fn row(&self, index: usize) -> &[T] {
        assert!(index <= self.ends.len());
        // TODO(perf): verify generated code elides bound checks.
        let get_end = |end: &u32| *end as usize;

        let start = index.checked_sub(1).map_or(0, |i| self.ends[i]) as usize;
        let end = self.ends.get(index).map_or(self.data.len(), get_end);
        &self.data[start..end]
    }
    /// Get `V` at exact `direct_index` ignoring row sizes,
    /// acts as if the whole array was a single row.
    ///
    /// `None` when `direct_index` is out of bound.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cuicui_datazoo::JaggedVec;
    ///
    /// let ends = [0, 0, 3, 4, 7, 9, 10, 10];
    /// let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// let jagged = JaggedVec::new(ends.to_vec(), data.to_vec()).unwrap();
    ///
    /// assert_eq!(jagged.get(4), Some(&4));
    /// ```
    #[inline]
    pub fn get(&self, direct_index: usize) -> Option<&T> {
        self.data.get(direct_index)
    }
    /// Turn this compact jagged array into a sparse representation.
    ///
    /// The returned `Vec<Vec<V>>` is an [Iliffe vector]. Iterating over it will
    /// be much slower than iterating over `JaggedVec`, but extending individual
    /// rows is much less costly.
    ///
    /// [Iliffe vector]: https://en.wikipedia.org/wiki/Iliffe_vector
    pub fn into_vecs(self) -> Vec<Vec<T>> {
        let Self { ends, mut data } = self;

        let mut iliffe = Vec::with_capacity(ends.len() + 1);
        let mut last_end = 0;

        // TODO(perf): this is slow as heck because each drain needs to move
        // forward the end of the `data` vec, if we reverse ends here, we can
        // skip the nonsense.
        for end in ends.into_iter() {
            let size = (end - last_end) as usize;
            iliffe.push(data.drain(..size).collect());
            last_end = end;
        }
        // the last row.
        iliffe.push(data);
        iliffe
    }
}
