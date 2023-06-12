We have a `binding::Id`, a small integer. And we have collections of:

`binding::Id` -> `Option<T>`

How to store `T`s with optimal memory usage, yet keeping lookups O(1) as an
index dereference?

## Sparse array

We could have a `[Option<T>; max(binding::Id)]`.

But supposing `size_of::<T>() > 2000`, that we have a single `T` and its binding
value is something like `100`, this is a lot of memory that exists to do about
exactly nothing!

## Sorted array

We currently use two different storing means for this:

- `BTreeMap<binding::Id, T>`: Works, is `O(log(n))`, and it's a fine thing given
  small size collection
- `SortedByKeyVec<binding::Id, T>`: This is still `O(log(n))`, better cache
  locality, not very good with insertions, but we generally don't care about
  insertions.

## Bit matrix map

Consider `BitMultimap`, it stores a `[K]`, a `[V]` and a `BitMatrix<Width=max(K), height=max(V)>`.

We could drop the `[K]` for `binding::Id` (we could even drop the `[V]` in the
case where we use it for `ModifyIndex`!) Directly using the `Id` value for
indexing. I mean, this is insane and reasonable at the same time.

But this is a _multimap_, it is made for situations where a single `K` can have
multiple `V`s.

Our situation describes a 1-to-0 relation. Not a n-to-m.

## Compact Index matrix

The idea of the `BitMatrix` would allocate `max(K)` rows of `max(V)` bits. In the case
of the 1-to-0 relation, either the row has a single bit enabled, or it has none.

Can this be done with less zeros?

Yes. Consider this: instead of representing a row with a bitset, we represent
it with a single integer.

We could use a `usize` as integer, but that would take 64 bits, only for a
target space of `max(V)`. If we know the upper bound of our target space (we do)
then, we can reduce the size of the integer (in bits) so that it can just
represent the largest index in the target space, and not more! 