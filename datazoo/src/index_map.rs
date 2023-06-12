//! An [associative arrays] of small integers.
//!
//! [associative arrays]: https://en.wikipedia.org/wiki/Associative_array

use std::{marker::PhantomData, mem};

use crate::{bitset::Ones, div_ceil, index_multimap::Index, Bitset};

/// An [associative arrays] of small integers.
///
/// This is a 1-to-(1|0) mapping, see [`IndexMultimap`] for N-to-M mapping.
///
/// The size in bytes of this `struct` is the lowest multiple of 4 over
/// `max(K) * log₂(max(V) + 1) / 8`
///
/// You'll notice the size is not dependent on the number of values stored
/// (in fact, [`IndexMap`] **does not** store any value). But rather the
/// values being stored themselves.
///
/// It is not recommended to use this data structure if you expect to have
/// large values in your key/value space.
///
/// [`IndexMap`] might be a good solution if you have an index to a small
/// array or an incrementing counter.
///
/// # Example
///
/// ```
/// todo!()
/// ```
///
/// [`IndexMultimap`]: crate::IndexMultimap
#[derive(Debug, Clone)]
pub struct IndexMap<K: Index, V: From<u32>> {
    /// A matrix of `max(K)` rows of `log₂(max(V) + 1)` bits, each row represents
    /// an index.
    ///
    /// If all the bits of the row are set, then it means the row is **empty**.
    /// (this allows `Value` values of 0)
    indices: Bitset<Box<[u32]>>,
    value_width: usize,
    _tys: PhantomData<fn(K, V)>,
}
impl<K: Index, V: From<u32>> IndexMap<K, V> {
    /// You may **not** insert values or keys larger than those parameters.
    pub fn new_with_size(max_value: u32, max_key: usize) -> Self {
        let height = max_key;
        let value_width = (u32::BITS - (max_value + 1).leading_zeros()) as usize;
        let bit_size = value_width * height;
        let u32_size = div_ceil(bit_size, mem::size_of::<u32>());
        IndexMap {
            indices: Bitset(vec![u32::MAX; u32_size].into_boxed_slice()),
            value_width,
            _tys: PhantomData,
        }
    }
    fn row_index(&self, index: &K) -> usize {
        index.get() * self.value_width
    }
    const fn value_mask(&self) -> u32 {
        (1 << self.value_width) - 1
    }
    /// Get the value associated with `index`, `None` if there isn't.
    pub fn get(&self, index: &K) -> Option<V> {
        let value = self.indices.u32_at(self.row_index(index))?;
        let value = value & self.value_mask();
        // != means the row is not empty
        (value != self.value_mask()).then(|| V::from(value))
    }
    /// Remove value associated with `key`. Calling `map.get(key)` will return
    /// `None`.
    pub fn remove(&mut self, key: &K) {
        let offset = self.row_index(key);
        self.indices.extend(offset..offset + self.value_width);
    }
    /// Set value of `key` to `value`.
    pub fn set(&mut self, key: &K, value: &V)
    where
        V: Index,
    {
        let value = value.get() as u32;
        let offset = self.row_index(key);
        let iter = Ones::from_single(value).map(|v| v + offset as u32);

        self.indices
            .disable_range(offset..offset + self.value_width);
        self.indices.extend(iter);
    }
}
impl<K: Index, V: From<u32> + Index> FromIterator<(K, V)> for IndexMap<K, V> {
    /// Create a [`IndexMap`] where value at `k` will be `value` in `(key, value)`
    /// the last item where `key == k`.
    ///
    /// Note that all `K` and `V` will be dropped.
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut max_value = 0;
        let mut max_key = 0;

        let key_values = iter
            .into_iter()
            .map(|(k, v)| {
                max_key = max_key.max(k.get());
                max_value = max_value.max(v.get());
                (k, v)
            })
            .collect::<Box<[_]>>();

        let max_value = u32::try_from(max_value).unwrap();
        let mut map = IndexMap::new_with_size(max_value, max_key);

        for (key, value) in key_values.iter() {
            map.set(key, value);
        }
        map
    }
}
