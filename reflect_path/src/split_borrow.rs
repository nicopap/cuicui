use std::mem::{self, MaybeUninit};

use bevy::reflect::{ParsedPath, Reflect};

use crate::atom::AtomicAccess;
use crate::parse::parse_path;

// # Safety
// - Do not create a second mutable reference to the same accessed path from
//   the returned `source`.
pub unsafe fn split_element_out<'a, T: Reflect + 'a>(
    path: &mut ParsedPath,
    source: &'a mut (dyn Reflect + 'static),
) -> (&'a mut (dyn Reflect + 'static), &'a mut T) {
    let field = path.element_mut::<T>(source).unwrap();
    let field_static = field as *mut T;
    (source, unsafe { field_static.as_mut().unwrap_unchecked() })
}

pub trait SplitPathTarget<'a, const N: usize>: Sized + sealed::SplitPathTarget<'a, N> {}
impl<'a, T: sealed::SplitPathTarget<'a, N>, const N: usize> SplitPathTarget<'a, N> for T {}

mod sealed {
    use super::*;

    pub trait SplitPathTarget<'a, const N: usize>: Sized {
        fn split(paths: &mut Multipath<N>, source: &'a mut (dyn Reflect + 'static))
            -> Option<Self>;
    }
    macro_rules! impl_split_path_target {
        ($n:tt $([$t_i:ident, $p_i:ident, $i:tt]),*) => {
            #[allow(unused_parens, unused_variables)]
            impl<'a, $($t_i: Reflect),*> SplitPathTarget<'a, $n> for ($(&'a mut $t_i),*) {
                fn split(
                    multipath: &mut Multipath<$n>,
                    source: &'a mut (dyn Reflect + 'static),
                ) -> Option<Self> {
                    $(
                        // SAFETY: `Multipath` guarentees each path within is mutually exclusive.
                        let (source, $p_i) = unsafe { split_element_out(&mut multipath.0[$i], source) };
                    )*
                    Some(($($p_i),*))
                }
            }
        };
    }
    impl_split_path_target!(1 [T0, p0, 0]);
    impl_split_path_target!(2 [T0, p0, 0], [T1, p1, 1]);
    impl_split_path_target!(3 [T0, p0, 0], [T1, p1, 1], [T2, p2, 2]);
    impl_split_path_target!(4 [T0, p0, 0], [T1, p1, 1], [T2, p2, 2], [T3, p3, 3]);
}

/// List of indices in `paths` that share data, therefore excludes
/// a split mutable borrow.
pub struct Incompatible;

pub struct Multipath<const N: usize>([ParsedPath; N]);

impl<const N: usize> Multipath<N> {
    pub fn new(paths: [&str; N]) -> Result<Self, Incompatible> {
        let mut parsed: [MaybeUninit<ParsedPath>; N] =
            unsafe { MaybeUninit::uninit().assume_init() };
        for (i, to_set) in parsed.iter_mut().enumerate() {
            // TODO(err)
            let parsed = ParsedPath::parse(paths[i]).unwrap();
            to_set.write(parsed);
        }
        // SAFETY: MaybeUninit<T> can be transmuted to T
        let parsed = parsed.map(|p| unsafe { mem::transmute::<_, ParsedPath>(p) });

        let mut atomic = AtomicAccess::with_capacity(N);
        for path in paths {
            // SAFETY: We return earlier if `ParsedPath::parse` fails.
            let parsed = unsafe { parse_path(path) };
            if atomic.add_path(parsed) {
                return Err(Incompatible);
            }
        }
        Ok(Multipath(parsed))
    }
    pub fn split<'r, T: SplitPathTarget<'r, N>>(
        &mut self,
        source: &'r mut (dyn Reflect + 'static),
    ) -> Option<T> {
        T::split(self, source)
    }
}

/// Create multiple `&'mut` views of a given `root: &mut dyn Reflect`
pub struct PathBorrower<'p, 'fld> {
    shared: AtomicAccess<'p>,
    root: Option<&'fld mut (dyn Reflect + 'static)>,
}
impl<'p, 'fld> Default for PathBorrower<'p, 'fld> {
    fn default() -> Self {
        PathBorrower { shared: AtomicAccess::new(), root: None }
    }
}
impl<'p, 'fld> PathBorrower<'p, 'fld> {
    pub fn new(root: &'fld mut (dyn Reflect + 'static)) -> Self {
        PathBorrower { shared: AtomicAccess::new(), root: Some(root) }
    }
    fn access_owned<T: Reflect>(self, path: &'p str) -> (Self, Option<&'fld mut T>) {
        let Self { mut shared, root } = self;

        let Ok(mut parsed) = ParsedPath::parse(path) else {
            return (Self { shared, root }, None);
        };
        // SAFETY: just parsed it earlier
        let exclusive = unsafe { parse_path(path) };
        if shared.add_path(exclusive) {
            return (Self { shared, root }, None);
        }
        // SAFETY: just checked earlier that we didn't already deliver a
        // reference
        let root = unsafe { root.unwrap_unchecked() };
        let (root, field) = unsafe { split_element_out(&mut parsed, root) };
        (Self { shared, root: Some(root) }, Some(field))
    }
    pub fn access<T: Reflect>(&mut self, path: &'p str) -> Option<&'fld mut T> {
        let (new_self, ret) = mem::take(self).access_owned(path);
        *self = new_self;
        ret
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[derive(Reflect)]
    struct A {
        w: usize,
        x: B,
        y: Vec<C>,
        z: D,
        unit_variant: F,
        tuple_variant: F,
        struct_variant: F,
        array: [i32; 3],
        tuple: (bool, f32),
    }

    #[derive(Reflect)]
    struct B {
        foo: usize,
        bar: C,
    }

    #[derive(Reflect, FromReflect)]
    struct C {
        baz: f32,
    }

    #[derive(Reflect)]
    struct D(E);

    #[derive(Reflect)]
    struct E(f32, usize);

    #[derive(Reflect, FromReflect, PartialEq, Debug)]
    enum F {
        Unit,
        Tuple(u32, u32),
        Struct { value: char },
    }

    fn a_sample() -> A {
        A {
            w: 1,
            x: B { foo: 10, bar: C { baz: 22.341 } },
            y: vec![C { baz: 1.0 }, C { baz: 2.0 }],
            z: D(E(10.0, 42)),
            unit_variant: F::Unit,
            tuple_variant: F::Tuple(123, 321),
            struct_variant: F::Struct { value: 'm' },
            array: [86, 75, 309],
            tuple: (true, 1.23),
        }
    }

    #[test]
    fn path_borrower() {
        let mut a = a_sample();

        let mut borrower = PathBorrower::new(&mut a);

        let tuple_1 = borrower.access::<f32>(".tuple.1").unwrap();
        let tuple_0 = borrower.access::<bool>(".tuple.0").unwrap();
        let baz = borrower.access::<f32>(".y[0].baz").unwrap();
        *tuple_1 += *baz + (*tuple_0 as u8 as f32);
        *baz = -*tuple_1;
    }
    #[test]
    #[should_panic]
    fn borrower_later_parent_access_panics() {
        let mut a = a_sample();
        let mut borrower = PathBorrower::new(&mut a);

        let _ = borrower.access::<f32>(".tuple.1").unwrap();
        let _ = borrower.access::<(bool, f32)>(".tuple").unwrap();
    }
    #[test]
    #[should_panic]
    fn borrower_child_access_panics() {
        let mut a = a_sample();
        let mut borrower = PathBorrower::new(&mut a);

        let _ = borrower.access::<(bool, f32)>(".tuple").unwrap();
        let _ = borrower.access::<f32>(".tuple.1").unwrap();
    }
}
