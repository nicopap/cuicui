use std::{cmp::Ordering, iter::Peekable};

pub enum Ior<L, R> {
    Left(L),
    Right(R),
    Both(L, R),
}
impl<L> Ior<L, L> {
    pub fn prefer_left(self) -> L {
        match self {
            Ior::Left(l) => l,
            Ior::Right(r) => r,
            Ior::Both(l, _) => l,
        }
    }
    pub fn intersected(self) -> Option<L>
    where
        L: PartialEq,
    {
        match self {
            Ior::Left(_) | Ior::Right(_) => None,
            Ior::Both(l, r) if l != r => None,
            Ior::Both(l, _) => Some(l),
        }
    }
    pub fn only_left(self) -> Option<L> {
        match self {
            Ior::Left(l) => Some(l),
            Ior::Right(_) | Ior::Both(_, _) => None,
        }
    }
}
pub struct JoinedSort<L: Iterator, R: Iterator, F> {
    left: Peekable<L>,
    right: Peekable<R>,
    cmp_fn: F,
}
impl<L, R, F> Iterator for JoinedSort<L, R, F>
where
    L: Iterator,
    R: Iterator,
    F: FnMut(&L::Item, &R::Item) -> Ordering,
{
    type Item = Ior<L::Item, R::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.left.peek(), self.right.peek()) {
            (None, None) => None,
            (Some(_), None) => Some(Ior::Left(self.left.next().unwrap())),
            (None, Some(_)) => Some(Ior::Right(self.right.next().unwrap())),
            (Some(left), Some(right)) => match (self.cmp_fn)(left, right) {
                Ordering::Equal => Some(Ior::Both(
                    self.left.next().unwrap(),
                    self.right.next().unwrap(),
                )),
                Ordering::Less => Some(Ior::Left(self.left.next().unwrap())),
                Ordering::Greater => Some(Ior::Right(self.right.next().unwrap())),
            },
        }
    }
}
pub fn joined_sort<L, R, F>(left: L, right: R, cmp_fn: F) -> JoinedSort<L, R, F>
where
    L: Iterator,
    R: Iterator,
    F: FnMut(&L::Item, &R::Item) -> Ordering,
{
    JoinedSort {
        left: left.peekable(),
        right: right.peekable(),
        cmp_fn,
    }
}
pub fn left_not_right<L, R, F>(left: L, right: R, cmp_fn: F) -> impl Iterator<Item = L::Item>
where
    L: IntoIterator,
    R: IntoIterator<Item = L::Item>,
    F: FnMut(&L::Item, &R::Item) -> Ordering,
{
    JoinedSort {
        left: left.into_iter().peekable(),
        right: right.into_iter().peekable(),
        cmp_fn,
    }
    .filter_map(Ior::only_left)
}
