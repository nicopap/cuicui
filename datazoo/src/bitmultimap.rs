use std::collections::BinaryHeap;

use super::bitmatrix::BitMatrix;

/// A sparse associative array.
///
/// This is a [multimap] with very good perf for small sets of key and values
/// that themselves shouldn't take much memory.
///
/// Furthermore, you can not only get all `values` associated with a given `key`,
/// but also all `keys` associated with a given `value`.
/// See [`BitMultiMap::get_keys_of`] and [`BitMultiMap::get`].
///
/// When the #K × #V > 8000, should consider using roaring bitmaps for this.
///
/// [multimap]: https://en.wikipedia.org/wiki/Multimap
#[derive(Debug)]
pub struct BitMultiMap<K: Eq + Ord, V: Eq + Ord> {
    sparse_keys: Box<[K]>,
    sparse_values: Box<[V]>,
    // TODO(feat): When the nº Modify that have Modify dependencies become very
    // large, we should consider a roaring bitmap
    // TODO(perf): Also consider storing a raw pointer with no size data,
    // since height + width are already stored in sparse_{keys,values}.
    // We could also merge all the arrays, this avoids fragmentation.
    /// A `sparse_keys.len()` × `sparse_values.len()` bitfield array.
    ///
    /// Each row is "all the `V`s associated with `K` at index k in `sparse_keys`"
    /// Specifically, the column is the index in `sparse_values` of relevant `V`s.
    associations: BitMatrix,
}
impl<K: Eq + Ord, V: Eq + Ord> BitMultiMap<K, V> {
    fn mapped_associates_of(&self, row: usize) -> impl Iterator<Item = usize> + '_ {
        let width = self.sparse_values.len();
        self.associations.row(width, row)
    }
    #[must_use]
    pub const fn keys(&self) -> &[K] {
        &self.sparse_keys
    }
    #[must_use]
    pub const fn values(&self) -> &[V] {
        &self.sparse_values
    }
    pub fn get(&self, key: &K) -> impl Iterator<Item = &V> + '_ {
        let mapped = self.sparse_keys.binary_search(key).ok();

        mapped
            .into_iter()
            .flat_map(|mapped| self.mapped_associates_of(mapped))
            // TODO(perf): This can be a `get_unchecked`
            .map(|mapped| &self.sparse_values[mapped])
    }
    pub fn get_keys_of(&self, value: &V) -> impl Iterator<Item = &K> + '_ {
        let mapped = self.sparse_values.binary_search(value).ok();
        let width = self.sparse_values.len();

        mapped
            .into_iter()
            .flat_map(move |mapped| self.associations.active_rows_in_column(width, mapped))
            // TODO(perf): This can be a `get_unchecked`
            .map(|mapped| &self.sparse_keys[mapped])
    }
}
impl<K: Eq + Ord + Clone, V: Eq + Ord + Clone> FromIterator<(K, V)> for BitMultiMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut values = BinaryHeap::new();
        let mut keys = BinaryHeap::new();

        let mut key_values = Vec::new();

        for (key, value) in iter {
            key_values.push((key.clone(), value.clone()));
            keys.push(key);
            values.push(value);
        }
        let sparse_keys: Box<[_]> = keys.into_sorted_vec().into();
        let sparse_values: Box<[_]> = values.into_sorted_vec().into();

        let mut associations = BitMatrix::new_with_size(sparse_values.len(), sparse_keys.len());

        for (key, value) in key_values {
            let key_i = sparse_keys.binary_search(&key).unwrap();
            let value_i = sparse_values.binary_search(&value).unwrap();

            associations
                .enable_bit(sparse_values.len(), key_i, value_i)
                .unwrap();
        }
        BitMultiMap { sparse_keys, sparse_values, associations }
    }
}
