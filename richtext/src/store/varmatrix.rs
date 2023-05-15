use enumset::{EnumSet, EnumSetType, __internal::EnumSetTypePrivate};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "Cannot build VarMatrix: `ends` represents the end of each row in `data`, \
        it should be monotonically increasing. \
        Found `end` at position {i} lower than `end` at position {}", .i - 1
    )]
    BadEnd { i: usize },
    #[error(
        "Cannot build VarMatrix: `ends` represents the end of each row in `data`, \
        Yet, `end` at position {i} ({end}) is larger than the length of data ({len})"
    )]
    TooLongEnd { i: usize, len: u32, end: u32 },
}

/// A variable length matrix, or irregular matrix, or jagged array, or Iliffe vector,
/// but optimized for read-only rows and statically known row count.
// TODO(perf): store the row indices inline, preventing cache misses when looking up several rows.
#[derive(Debug)]
pub struct VarMatrix<V, const R: usize> {
    ends: Box<[u32; R]>,
    data: Box<[V]>,
}

impl<V, const R: usize> VarMatrix<V, R> {
    pub fn new(ends: Box<[u32; R]>, data: Box<[V]>) -> Result<Self, Error> {
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
            .map(|u32| usize::try_from(u32).unwrap())
            .flat_map(|elem| self.row(elem).iter())
    }
    pub fn row(&self, index: usize) -> &[V] {
        assert!(index < R + 1);
        let get_end = |end: &u32| *end as usize;

        let start = index.checked_sub(1).map_or(0, |i| self.ends[i]) as usize;
        let end = self.ends.get(index).map_or(self.data.len(), get_end);
        &self.data[start..end]
    }
    pub fn get(&self, direct_index: usize) -> Option<&V> {
        self.data.get(direct_index)
    }
}
