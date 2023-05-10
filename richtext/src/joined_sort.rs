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
