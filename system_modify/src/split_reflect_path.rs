use bevy::reflect::{ParsedPath, Reflect};

// # Safety
// - Do not create a second mutable reference to the same accessed path from
//   the returned `source`.
unsafe fn split_element_out<'a, T: Reflect + 'a>(
    path: &mut ParsedPath,
    source: &'a mut (dyn Reflect + 'static),
) -> (&'a mut (dyn Reflect + 'static), &'a mut T) {
    let field = path.element_mut::<T>(source).unwrap();
    let field_static = field as *mut T;
    (source, unsafe { field_static.as_mut().unwrap_unchecked() })
}

pub trait SplitPathTarget<'a, const N: usize>: Sized {
    fn split(paths: Multipath<N>, source: &'a mut (dyn Reflect + 'static)) -> Option<Self>;
}
impl<'a, T0: Reflect, T1: Reflect, T2: Reflect> SplitPathTarget<'a, 3>
    for (&'a mut T0, &'a mut T1, &'a mut T2)
{
    fn split(
        Multipath([p0, p1, p2]): Multipath<3>,
        source: &'a mut (dyn Reflect + 'static),
    ) -> Option<Self> {
        let (source, p0) = unsafe { split_element_out(p0, source) };
        let (source, p1) = unsafe { split_element_out(p1, source) };
        let (_, p2) = unsafe { split_element_out(p2, source) };
        Some((p0, p1, p2))
    }
}

pub struct Multipath<'a, const N: usize>(&'a mut [ParsedPath; N]);

impl<'a, const N: usize> Multipath<'a, N> {
    pub fn new(paths: &'a mut [ParsedPath; N]) -> Self {
        Multipath(paths)
    }
    pub fn split<'r, T: SplitPathTarget<'r, N>>(
        self,
        source: &'r mut (dyn Reflect + 'static),
    ) -> Option<T> {
        T::split(self, source)
    }
}
