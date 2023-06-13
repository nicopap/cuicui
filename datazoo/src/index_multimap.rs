//! A [multimap] that goes from an integer to multiple integers.
//!
//! [multimap]: https://en.wikipedia.org/wiki/Multimap
use std::marker::PhantomData;

use crate::{BitMatrix, Index};

/// A [multimap] that goes from an integer to multiple integers.
///
/// This is a 1-to-N mapping, see [`RawIndexMap`] for 1-to-(1|0) mapping.
/// [`JaggedBitset`] is an alternative in case you expect the largest
/// row to be way larger than the smaller ones.
///
/// The size in bytes of this `struct` is the lowest multiple of 4 over
/// `max(K) * max(V) / 8`
///
/// You'll notice the size is not dependent on the number of values stored
/// (in fact, [`IndexMultimap`] **does not** store any value). But rather the
/// values being stored themselves.
///
/// It is not recommended to use this data structure if you expect to have
/// large values in your key/value space.
///
/// [`IndexMultimap`] might be a good solution if you have an index to a small
/// array or an incrementing counter.
///
/// # Example
///
/// ```
/// use cuicui_datazoo::IndexMultimap;
///
/// let multimap: IndexMultimap<usize, usize> = [
///     (0, 1), (0, 5), (0, 2), (0, 2),
///     (1, 7), (1, 0), (1, 1),
///     (2, 32), (2, 0), (2, 12), (2, 2), (2, 11), (2, 10), (2, 13), (2, 4),
///     (4, 1)
/// ].into_iter().collect();
/// let rows: [&[usize]; 5] = [
///     &[1, 2, 5],
///     &[0, 1, 7],
///     &[0, 2, 4, 10, 11, 12, 13, 32],
///     &[],
///     &[1],
/// ];
/// for (i, row) in rows.iter().enumerate() {
///     let multimap_row: Box<[usize]> = multimap.get(&i).collect();
///     assert_eq!(*row, &*multimap_row, "{i}");
/// }
/// ```
///
/// [`IndexMap`]: crate::IndexMap
/// [`JaggedBitset`]: crate::JaggedBitset
#[derive(Debug, Clone)]
pub struct IndexMultimap<K: Index, V: From<usize>> {
    assocs: BitMatrix,
    value_count: usize,
    _idx_ty: PhantomData<fn(K, V)>,
}
impl<K: Index, V: From<usize>> IndexMultimap<K, V> {
    /// Get the values associated with given `K`
    pub fn get<'a>(&'a self, key: &K) -> impl Iterator<Item = V> + 'a {
        let index = key.get();
        let max_index = self.assocs.height(self.value_count);
        (max_index > index)
            .then(|| self.assocs.row(self.value_count, index).map(|i| V::from(i)))
            .into_iter()
            .flatten()
    }
}
impl<K: Index, V: From<usize> + Index> FromIterator<(K, V)> for IndexMultimap<K, V> {
    /// Create a [`IndexMultimap`] with all associations.
    ///
    /// Note that `K` and `V` will be dropped.
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut max_value = 0;
        let mut max_key = 0;

        let key_values = iter
            .into_iter()
            .map(|(k, v)| {
                max_key = max_key.max(k.get() + 1);
                max_value = max_value.max(v.get() + 1);
                (k, v)
            })
            .collect::<Box<[_]>>();

        let (width, height) = (max_value, max_key);
        let mut assocs = BitMatrix::new_with_size(width, height);

        for (key, value) in key_values.iter() {
            assocs.enable_bit(width, value.get(), key.get()).unwrap();
        }
        IndexMultimap { assocs, value_count: width, _idx_ty: PhantomData }
    }
}
