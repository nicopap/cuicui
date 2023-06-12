//! A [multimap] that goes from an integer to multiple integers.
//!
//! [multimap]: https://en.wikipedia.org/wiki/Multimap
use std::marker::PhantomData;

use crate::BitMatrix;

/// Get an `usize` from `Self`.
pub trait Index {
    fn get(&self) -> usize;
}
impl Index for usize {
    fn get(&self) -> usize {
        *self
    }
}

/// A [multimap] that goes from an integer to multiple integers.
///
/// This is a N-to-M mapping, see [`IndexMap`] for 1-to-(1|0) mapping.
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
/// todo!()
/// ```
///
/// [`IndexMap`]: crate::IndexMap
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
