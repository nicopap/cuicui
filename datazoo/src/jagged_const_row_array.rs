//! A variable length matrix optimized for read-only
//! rows and statically known row count.

use std::mem::size_of;

use enumset::{EnumSet, EnumSetType, __internal::EnumSetTypePrivate};

use thiserror::Error;

/// [`JaggedConstRowArray::new`] construction error.
#[derive(Debug, Error)]
pub enum Error {
    /// An `end` in `ends` was lower than a previous one.
    #[error(
        "Cannot build JaggedConstRowArray: `ends` represents the end of each row in `data`, \
        it should be monotonically increasing. \
        Found `end` at position {i} lower than `end` at position {}", .i - 1
    )]
    BadEnd { i: usize },
    /// An `end` in `ends` was too large.
    #[error(
        "Cannot build JaggedConstRowArray: `ends` represents the end of each row in `data`, \
        Yet, `end` at position {i} ({end}) is larger than the length of data ({len})"
    )]
    TooLongEnd { i: usize, len: u32, end: u32 },
}

/// A variable length matrix optimized for read-only rows and statically known row count.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct JaggedConstRowArray<V, const R: usize> {
    // TODO(perf): store the row indices inline, preventing cache misses when looking up several rows.
    ends: Box<[u32; R]>,
    data: Box<[V]>,
}

impl<V, const R: usize> JaggedConstRowArray<V, R> {
    /// How many cells are contained in this `JaggedConstRowArray`.
    pub const fn len(&self) -> usize {
        self.data.len()
    }
    /// Is this array empty (no cells, may have several empty rows).
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    /// How many rows this `JaggedConstRowArray` has.
    pub const fn height(&self) -> usize {
        self.ends.len() + 1
    }
    /// Create a [`JaggedConstRowArray`] of `R + 1` rows, values of `ends` are the
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
    /// use cuicui_datazoo::JaggedConstRowArray;
    ///
    /// let ends = [0, 0, 3, 4, 7, 9, 10, 10];
    /// let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 32];
    /// let jagged = JaggedConstRowArray::new(Box::new(ends), Box::new(data)).unwrap();
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
    ///         vec![11, 32],
    ///     ],
    /// );
    /// ```
    pub fn new(ends: Box<[u32; R]>, data: Box<[V]>) -> Result<Self, Error> {
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
    pub(super) fn all_rows<T: EnumSetType>(&self, set: EnumSet<T>) -> impl Iterator<Item = &V> {
        set.iter()
            .map(EnumSetTypePrivate::enum_into_u32)
            .flat_map(|elem| self.row(elem as usize).iter())
    }
    /// Get slice to row at given `index`.
    ///
    /// # Panics
    ///
    /// When `index > R`
    ///
    /// # Example
    ///
    /// ```rust
    /// use cuicui_datazoo::JaggedConstRowArray;
    ///
    /// let ends = [0, 0, 3, 4, 7, 9, 10, 10];
    /// let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// let jagged = JaggedConstRowArray::new(Box::new(ends), Box::new(data)).unwrap();
    ///
    /// assert_eq!(jagged.row(4), &[4, 5, 6]);
    /// ```
    #[inline]
    pub fn row(&self, index: usize) -> &[V] {
        assert!(index <= R);
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
    /// use cuicui_datazoo::JaggedConstRowArray;
    ///
    /// let ends = [0, 0, 3, 4, 7, 9, 10, 10];
    /// let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// let jagged = JaggedConstRowArray::new(Box::new(ends), Box::new(data)).unwrap();
    ///
    /// assert_eq!(jagged.get(4), Some(&4));
    /// ```
    #[inline]
    pub fn get(&self, direct_index: usize) -> Option<&V> {
        self.data.get(direct_index)
    }
    /// Turn this compact jagged array into a sparse representation.
    ///
    /// The returned `Vec<Vec<V>>` is an [Iliffe vector]. Iterating over it will
    /// be much slower than iterating over `JaggedConstRowArray`, but extending individual
    /// rows is much less costly.
    ///
    /// [Iliffe vector]: https://en.wikipedia.org/wiki/Iliffe_vector
    pub fn into_vecs(self) -> Vec<Vec<V>> {
        let Self { ends, data } = self;
        let mut data = data.into_vec();

        let mut iliffe = Vec::with_capacity(ends.len());
        let mut last_end = 0;

        // TODO(perf): this is slow as heck because each drain needs to move
        // forward the end of the `data` vec, if we reverse ends here, we can
        // skip the nonsense.
        for end in ends.into_iter() {
            let size = (end - last_end) as usize;
            iliffe.push(data.drain(..size).collect());
            last_end = end;
        }
        iliffe.push(data);
        iliffe
    }
}
