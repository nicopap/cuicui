use std::marker::PhantomData;

use datazoo::BitMatrix;

pub trait Index {
    fn get(&self) -> usize;
    fn new(index: usize) -> Self;
}

#[derive(Debug, Clone)]
pub struct IndexMultimap<Idx> {
    assocs: BitMatrix,
    width: usize,
    _idx_ty: PhantomData<Idx>,
}
impl<I: Index> IndexMultimap<I> {
    pub(crate) fn get(&self, index: I) -> impl Iterator<Item = I> + '_ {
        self.assocs.row(self.width, index.get()).map(|i| I::new(i))
    }
}
impl<I: Index> FromIterator<(I, I)> for IndexMultimap<I> {
    /// Create a [`IndexMultimap`] with all associations.
    fn from_iter<T: IntoIterator<Item = (I, I)>>(iter: T) -> Self {
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
        IndexMultimap { assocs, width, _idx_ty: PhantomData::<I> }
    }
}
