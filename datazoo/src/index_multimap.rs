use std::marker::PhantomData;

use crate::BitMatrix;

/// Get an `usize` from `Self`.
pub trait Index {
    fn get(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct IndexMultimap<K: Index, V: From<usize>> {
    assocs: BitMatrix,
    width: usize,
    _idx_ty: PhantomData<fn(K, V)>,
}
impl<K: Index, V: From<usize>> IndexMultimap<K, V> {
    pub fn get(&self, index: K) -> impl Iterator<Item = V> + '_ {
        self.assocs.row(self.width, index.get()).map(|i| V::from(i))
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
                max_key = max_key.max(k.get());
                max_value = max_value.max(v.get());
                (k, v)
            })
            .collect::<Box<[_]>>();

        let (width, height) = (max_value, max_key);
        let mut assocs = BitMatrix::new_with_size(width, height);

        for (key, value) in key_values.iter() {
            assocs.enable_bit(width, key.get(), value.get()).unwrap();
        }
        IndexMultimap { assocs, width, _idx_ty: PhantomData }
    }
}
