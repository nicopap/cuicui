//! An associative array without hashing.

use std::marker::PhantomData;

use crate::{index_multimap::Index, Bitset};

/// An [associative arrays] of small integers.
///
/// This is a 1-to-(1|0) mapping, see [`IndexMultimap`] for N-to-M mapping.
///
/// The size in bytes of this `struct` is the lowest multiple of 4 over
/// `max(K) * log₂(len(V) + 1) / 8`
///
/// You'll notice the size depends on the max value of `K`.
///
/// It is not recommended to use this data structure if you expect to have
/// large values in your key space.
///
/// # Example
///
/// ```
/// todo!()
/// ```
///
/// [`IndexMultimap`]: crate::IndexMultimap
pub struct IndexMap<K: Index, V> {
    /// A matrix of `max(K)` rows of `log₂(len(V) + 1)` bits, each row represents
    /// an index.
    ///
    /// If all the bits of the row are set, then it means the row is **empty**.
    indices: Bitset<Vec<u32>>,
    values: Vec<V>,
    _tys: PhantomData<fn(K)>,
}
impl<K: Index, V> IndexMap<K, V> {
    pub fn new() -> Self {
        IndexMap {
            indices: Bitset(Vec::new()),
            values: Vec::new(),
            _tys: PhantomData,
        }
    }
    fn offset_width(&self) -> usize {
        (u32::BITS - self.values.len().leading_zeros()) as usize
    }
    fn offset_of(&self, key: &K) -> usize {
        key.get() * self.offset_width()
    }
    fn value_mask(&self) -> u32 {
        (1 << self.offset_width()) - 1
    }
    fn get_index(&self, key: &K) -> Option<usize> {
        let value = self.indices.u32_at(self.offset_of(key))?;
        let value = value & self.value_mask();
        // != means the row is not empty
        (value != self.value_mask()).then_some(value as usize)
    }
    fn push(&mut self, key: &K, value: V) {
        let offset = self.offset_of(key);
        let iter = Ones::from_single(value).map(|v| v + offset as u32);
        self.indices
            .disable_range(offset..offset + self.value_width);
        self.indices.extend(iter);
    }
    /// Get the value associated with `index`, `None` if there isn't.
    pub fn get<'a>(&'a self, key: &K) -> Option<&'a V> {
        let index = self.get_index(key)?;
        // TODO(perf): may be able to assume Some
        self.values.get(index)
    }
    /// Remove value associated with `key`. Afterward, calling `map.get(key)`
    /// will return `None`.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let index = self.get_index(index)?;
        let offset = self.offset_of(key);
        self.indices.extend(offset..offset + self.value_width);
    }
    /// Set value of `key` to `value`.
    ///
    /// Returns the previous value if any.
    ///
    /// This is fairly costly if `key` didn't already have
    pub fn set(&mut self, key: &K, value: V) -> Option<V> {
        match self.get_index(index) {
            Some(pre_existing) => mem::replace(&mut self.values[pre_existing], value),
            None => self.push(key, value),
        }
    }
}
impl<K: Index, V> FromIterator for IndexMap<K, V> {
    /// Create a [`IndexMap`] where value at `k` will be `value` in `(key, value)`
    /// the last item where `key == k`.
    ///
    /// Note that all `K`s and duplicate `V`s will be dropped.
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
