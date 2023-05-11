use enumset::{EnumSet, EnumSetType, __internal::EnumSetTypePrivate};

use crate::bitset::BitSetExtensions;

// FIXME(clean): I should turns this into a `K, V` where `K: EnumSetTypePrivate`.
pub struct Varmatrix<V, const CLM: usize> {
    // FIXME(perf): when rust allows {CLM-1} in const generic param, this
    // can be replaced, as the first will always be 0 in our naive implementation.
    start_at: [u32; CLM],
    data: [V],
}
impl<V, const CLM: usize> Varmatrix<V, CLM> {
    pub fn all_rows<T: EnumSetType>(&self, set: EnumSet<T>) -> impl Iterator<Item = &V> {
        assert_eq!(T::VARIANT_COUNT as usize, CLM);
        set.iter()
            .map(EnumSetTypePrivate::enum_into_u32)
            .map(|u32| usize::try_from(u32).unwrap())
            .filter_map(|usize| (usize < CLM).then_some(usize))
            .flat_map(|elem| self.row(elem).iter())
    }
    pub fn row(&self, index: usize) -> &[V] {
        assert!(index < CLM);
        let start = self.start_at[index] as usize;
        match self.start_at.get(index + 1) {
            Some(&end) => &self.data[start..end as usize],
            None => &self.data[start..],
        }
    }
}

pub struct Assoc {
    sparse_input_index: Box<[u32]>,
    sparse_output_index: Box<[u32]>,
    // TODO(feat): When the nº Modify that have Modify dependencies become very
    // large, we should consider a roaring bitmap
    // TODO(perf): Also consider storing a raw pointer with no size data,
    // since height + width are already stored in sparse_{input,output}.
    associations: Box<[u32]>,
}
impl Assoc {
    fn mapped_associates_of(&self, row: usize) -> impl Iterator<Item = usize> + '_ {
        let width = self.sparse_output_index.len();
        let start = row * width;
        let end = (row + 1) * width;

        self.associations
            .ones_in_range(start..end)
            .map(move |i| (i as usize) - start)
    }

    pub fn associates(&self, index: u32) -> impl Iterator<Item = u32> + '_ {
        let mapped = self.sparse_input_index.iter().position(|i| *i == index);

        mapped
            .into_iter()
            .flat_map(|mapped| self.mapped_associates_of(mapped))
            // TODO(perf): This can be a `get_unchecked`
            .map(|mapped| self.sparse_output_index[mapped])
    }
}
