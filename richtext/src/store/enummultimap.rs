use std::marker::PhantomData;

use enumset::EnumSet;

use super::{VarMatrix, VarMatrixError};

pub use enumset::EnumSetType;

/// A MultiMap stored in a [`VarMatrix`].
///
/// The key set need to be bound and exhaustively known at compile time,
/// ie: it must be an enum derived with `#[derive(EnumSetType)]`.
///
/// Use it as follow:
/// `EnumMultiMap<MyEnumSet, ModifyIndex, { (MyEnumSet::BIT_WIDTH - 1) as usize }>`
#[derive(Debug)]
pub struct EnumMultiMap<K: EnumSetType, V, const CLM: usize> {
    inner: VarMatrix<V, CLM>,
    _key: PhantomData<K>,
}
impl<K: EnumSetType, V, const CLM: usize> EnumMultiMap<K, V, CLM> {
    /// Compile time error when `CLM` is not the correct value.
    ///
    /// This works around a limitation of rust' type system,
    /// where it is impossible to use associated constants in generic const position.
    const SENSIBLE: () = {
        assert!(K::BIT_WIDTH as usize == CLM + 1);
    };
    pub fn all_rows(&self, set: EnumSet<K>) -> impl Iterator<Item = &V> {
        let () = Self::SENSIBLE;
        self.inner.all_rows(set)
    }
    pub fn row(&self, key: K) -> &[V] {
        let () = Self::SENSIBLE;
        self.inner.row(key.enum_into_u32() as usize)
    }
    pub fn get(&self, direct_index: usize) -> Option<&V> {
        let () = Self::SENSIBLE;
        self.inner.get(direct_index)
    }
}

pub struct EnumMultiMapBuilder<K, V, const CLM: usize> {
    // TODO(perf): could be replaced by eehh idk
    pub rows: Vec<Box<[V]>>,
    _key: PhantomData<K>,
}
impl<K: EnumSetType, V, const CLM: usize> EnumMultiMapBuilder<K, V, CLM> {
    pub fn new() -> Self {
        EnumMultiMapBuilder { rows: Vec::with_capacity(CLM), _key: PhantomData }
    }
    pub fn insert(&mut self, key: K, values: impl Iterator<Item = V>) {
        let row = key.enum_into_u32() as usize;
        self.rows.insert(row, values.collect());
    }
    pub fn build(self) -> Result<EnumMultiMap<K, V, CLM>, VarMatrixError> {
        let mut end = 0;
        let mut ends = Box::new([0; CLM]);
        let mut data = Vec::new();
        for (i, values) in self.rows.into_iter().enumerate() {
            end += values.len() as u32;
            data.extend(values.into_vec());
            if i < CLM {
                ends[i] = end;
            }
        }
        Ok(EnumMultiMap {
            inner: VarMatrix::new(ends, data.into_boxed_slice())?,
            _key: PhantomData,
        })
    }
}
