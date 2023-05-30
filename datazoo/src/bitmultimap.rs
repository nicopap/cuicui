use core::fmt;
use std::collections::BinaryHeap;

use sorted_iter::{assume::AssumeSortedByItemExt, SortedIterator};

use crate::sorted;

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
/// # Representation
///
/// Consider `K = char` and `V = i64`. A `BitMultiMap` stores a limited subset of
/// `char` and `i64` and an association list.
///
/// Keys and values are stored in two sorted lists, associations in a [bitset
/// matrix](BitMatrix).
///
/// #### Keys
///     
/// |E|G|H|M|S|T|
/// |-|-|-|-|-|-|
///
/// #### Values
///
/// |-5|-1|10|342|1024|
/// |--|--|--|---|----|
///
/// #### Associations
///
/// |    |E|G|H|M|S|T|
/// |----|-|-|-|-|-|-|
/// |-5  | | |█|█| |█|
/// |-1  |█|█| | | |█|
/// |10  | | |█|█| | |
/// |342 | |█|█| |█| |
/// |1024| | | | | |█|
///
/// ## Example
///
/// ```rust
/// use cuicui_datazoo::BitMultiMap;
///
/// let associations: Vec<(char, i64)> = vec![
///     ('E', -1),
///     ('G', -1), ('G', 342),
///     ('H', -5), ('H', 10), ('H', 342),
///     ('M', -5), ('M', 10),
///     ('S', 342),
///     ('T',-5), ('T',-1), ('T',1024),
/// ];
/// let map: BitMultiMap<char, i64> = associations.into_iter().collect();
///
/// let assocs = map.get(&'V').copied().collect::<Vec<_>>();
/// assert_eq!(&assocs, &[]);
///
/// let assocs = map.get(&'T').copied().collect::<Vec<_>>();
/// assert_eq!(&assocs, &[-5, -1, 1024]);
///
/// let assocs = map.get_keys_of(&-1).copied().collect::<Vec<_>>();
/// assert_eq!(&assocs, &['E', 'G', 'T']);
/// ```
///
/// [multimap]: https://en.wikipedia.org/wiki/Multimap
pub struct BitMultiMap<K: Eq + Ord, V: Eq + Ord> {
    sparse_keys: sorted::Box<K>,
    sparse_values: sorted::Box<V>,
    // TODO(feat): When the nº Modify that have Modify dependencies become very
    // large, we should consider a roaring bitmap (#K × #V > 8000)
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
    /// Return indices in `sparse_values` of values associated with key of index `row`.
    #[inline]
    fn mapped_associates_of(&self, row: usize) -> impl Iterator<Item = usize> + '_ {
        let width = self.sparse_values.len();
        self.associations.row(width, row)
    }
    /// All keys present in this map, sorted.
    #[must_use]
    pub fn keys(&self) -> sorted::Slice<K> {
        self.sparse_keys.slice()
    }
    /// All values present in this map, sorted.
    #[must_use]
    pub fn values(&self) -> sorted::Slice<V> {
        self.sparse_values.slice()
    }
    /// Get all values associated with `key`.
    pub fn get(&self, key: &K) -> impl SortedIterator<Item = &V> + '_ {
        self.sparse_keys
            .binary_search(key)
            .into_iter()
            .flat_map(|mapped| self.mapped_associates_of(mapped))
            // SAFETY: By construction, `mapped` will always be smaller than `#V`
            // because `associations` is constructed as BitMatrix::new_with_size(#V, #K)
            // and `mapped` is an index in a row (so, of max size #V).
            .map(|mapped| unsafe { self.sparse_values.get_unchecked(mapped) })
            .assume_sorted_by_item()
    }
    /// Get all keys associated with `value`.
    pub fn get_keys_of(&self, value: &V) -> impl SortedIterator<Item = &K> + '_ {
        let width = self.sparse_values.len();

        self.sparse_values
            .binary_search(value)
            .into_iter()
            .flat_map(move |mapped| self.associations.active_rows_in_column(width, mapped))
            // TODO(perf): This can be a `get_unchecked`
            .filter_map(|mapped| self.sparse_keys.get(mapped))
            .assume_sorted_by_item()
    }
}
impl<K: Eq + Ord + Clone, V: Eq + Ord + Clone> FromIterator<(K, V)> for BitMultiMap<K, V> {
    /// Create a [`BitMultiMap`] with all associations.
    ///
    /// Note that this takes into account
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut values = BinaryHeap::new();
        let mut keys = BinaryHeap::new();

        let mut key_values = Vec::new();

        for (key, value) in iter {
            key_values.push((key.clone(), value.clone()));
            keys.push(key);
            values.push(value);
        }
        let sparse_keys: sorted::Box<_> = keys.into();
        let sparse_values: sorted::Box<_> = values.into();

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

impl<K: Eq + Ord + fmt::Debug, V: Eq + Ord + fmt::Debug> fmt::Debug for BitMultiMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (width, height) = (self.sparse_values.len(), self.sparse_keys.len());
        f.debug_struct("BitMultiMap")
            .field("values", &self.sparse_values)
            .field("keys", &self.sparse_keys)
            .field("map", &self.associations.sextant_display(width, height))
            .finish()
    }
}
