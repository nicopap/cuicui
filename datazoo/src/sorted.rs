//! Types marking slices as being sorted.

use core::fmt;
use std::{collections::BinaryHeap, marker::PhantomData, ops::Deref, slice};

use sorted_iter::{sorted_iterator::SortedByItem, sorted_pair_iterator::SortedByKey};

/// A `Vec<(K, V)>` where all elements are sorted in ascending ordeorder according
/// to `K`.
pub type ByKeyVec<K, V> = KeySorted<std::vec::Vec<(K, V)>, K, V>;

/// A `Box<[(K, V)]>` where all elements are sorted in ascending ordeorder according
/// to `K`.
pub type ByKeyBox<K, V> = KeySorted<std::boxed::Box<[(K, V)]>, K, V>;

/// A `&'a [(K, V)]` where all elements are sorted in ascending ordeorder according
/// to `K`.
pub type ByKeySlice<'a, K, V> = KeySorted<&'a [(K, V)], K, V>;

/// A `Vec<T>` where all elements are sorted in ascending ordeorder according
/// to `T: Ord`.
pub type Vec<T> = Sorted<std::vec::Vec<T>, T>;

/// A `Box<[T]>` where all elements are sorted in ascending ordeorder according
/// to `T: Ord`.
pub type Box<T> = Sorted<std::boxed::Box<[T]>, T>;

/// A `&'a [T]` where all elements are sorted in ascending ordeorder according
/// to `T: Ord`.
pub type Slice<'a, T> = Sorted<&'a [T], T>;

// -------------------------
//          KeySorted
// -------------------------

/// Slices where all elements are key-value pairs sorted in ascending ordeorder
/// according to key's `K: Ord`.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct KeySorted<A: AsRef<[(K, V)]>, K: Ord, V>(A, PhantomData<(K, V)>);

impl<K: Ord, V, A: AsRef<[(K, V)]> + fmt::Debug> fmt::Debug for KeySorted<A, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("key-sorted").field(&self.0).finish()
    }
}
impl<K: Ord, V> From<std::vec::Vec<(K, V)>> for ByKeyVec<K, V> {
    fn from(mut value: std::vec::Vec<(K, V)>) -> Self {
        value.sort_unstable_by(|l, r| l.0.cmp(&r.0));
        Self(value, PhantomData)
    }
}
impl<K: Ord, V> From<std::vec::Vec<(K, V)>> for ByKeyBox<K, V> {
    fn from(value: std::vec::Vec<(K, V)>) -> Self {
        let value: ByKeyVec<_, _> = value.into();
        Self(value.0.into_boxed_slice(), PhantomData)
    }
}

impl<A: AsRef<[(K, V)]>, K: Ord, V> Deref for KeySorted<A, K, V> {
    type Target = [(K, V)];

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<A: AsRef<[(K, V)]>, K: Ord, V> KeySorted<A, K, V> {
    pub fn iter(&self) -> KeysortedIter<K, V> {
        KeysortedIter(self.0.as_ref().iter())
    }
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn into_inner(self) -> A {
        self.0
    }
}
pub struct KeysortedIter<'a, K, V>(slice::Iter<'a, (K, V)>);
impl<'a, K, V> Iterator for KeysortedIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k, v))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
    fn count(self) -> usize {
        self.0.count()
    }
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n).map(|(k, v)| (k, v))
    }
    fn fold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, mut f: F) -> B {
        self.0.fold(init, |acc, (k, v)| f(acc, (k, v)))
    }
}
impl<K, V> ExactSizeIterator for KeysortedIter<'_, K, V> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<K, V> SortedByKey for KeysortedIter<'_, K, V> {}

impl<A: AsRef<[(K, V)]> + FromIterator<(K, V)>, K: Ord, V> KeySorted<A, K, V> {
    /// Create a [`KeySorted`] collection from a sorted iterator.
    pub fn from_sorted_iter<I>(iter: I) -> Self
    where
        I: Iterator<Item = (K, V)> + SortedByKey,
    {
        KeySorted(iter.collect(), PhantomData)
    }
}

// -------------------------
//           Sorted
// -------------------------

/// Slices where all elements are sorted in ascending ordeorder according
/// to `T: Ord`.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Sorted<A: AsRef<[T]>, T: Ord>(A, PhantomData<T>);
impl<A: AsRef<[T]>, T: Ord> Sorted<A, T> {
    pub fn slice(&self) -> Slice<T> {
        Sorted(self.0.as_ref(), PhantomData)
    }
}

impl<T: Ord, A: AsRef<[T]> + fmt::Debug> fmt::Debug for Sorted<A, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("sorted").field(&self.0).finish()
    }
}
impl<T: Ord> From<std::vec::Vec<T>> for Vec<T> {
    fn from(mut value: std::vec::Vec<T>) -> Self {
        value.sort_unstable();
        Self(value, PhantomData)
    }
}
impl<T: Ord> From<std::vec::Vec<T>> for Box<T> {
    fn from(value: std::vec::Vec<T>) -> Self {
        let value: Vec<_> = value.into();
        Self(value.0.into_boxed_slice(), PhantomData)
    }
}
impl<T: Ord> From<BinaryHeap<T>> for Vec<T> {
    fn from(value: BinaryHeap<T>) -> Self {
        Sorted(value.into_sorted_vec(), PhantomData)
    }
}
impl<T: Ord> From<BinaryHeap<T>> for Box<T> {
    fn from(value: BinaryHeap<T>) -> Self {
        Sorted(value.into_sorted_vec().into_boxed_slice(), PhantomData)
    }
}

impl<A: AsRef<[T]>, T: Ord> Deref for Sorted<A, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
pub struct SortedIter<'a, T>(slice::Iter<'a, T>);
impl<T> SortedByItem for SortedIter<'_, T> {}

impl<A: AsRef<[T]> + FromIterator<T>, T: Ord> Sorted<A, T> {
    /// Create a [`Sorted`] collection from a sorted iterator.
    pub fn from_sorted_iter<I>(iter: I) -> Self
    where
        I: Iterator<Item = T> + SortedByItem,
    {
        Sorted(iter.collect(), PhantomData)
    }
}
