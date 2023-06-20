use std::marker::PhantomData;

use bevy::prelude::{Component, Mut};
use bevy::reflect::{ParsedPath, Reflect};

use crate::access_registry::CompAccessRecorder;
use crate::access_registry::FnAccessRecorder;
use crate::split_reflect_path::Multipath;

pub trait AccessField {
    type Concrete<'z>;
    type From: Reflect;
    const READ: bool;
    const WRITE: bool;
    fn new(value: &mut Self::From) -> Self::Concrete<'_>;
}

pub trait Access {
    type Concrete<'z>;
    type Paths: Send + Sync + 'static;
    type ParsedPaths: Send + Sync + 'static;

    fn record(paths: &Self::Paths, rec: &mut CompAccessRecorder);
    fn parse_path(paths: Self::Paths) -> Self::ParsedPaths;
    fn extract<'z, C: Component + Reflect>(
        value: Mut<'z, C>,
        path: &mut Self::ParsedPaths,
    ) -> Self::Concrete<'z>;
}

// -----
//
// impl AccessField
//
// -----

impl<T: Reflect> AccessField for &'_ T {
    type Concrete<'z> = &'z T;
    type From = T;
    const READ: bool = true;
    const WRITE: bool = false;
    fn new(value: &mut T) -> &T {
        value
    }
}
impl<T: Reflect> AccessField for &'_ mut T {
    type Concrete<'z> = &'z mut T;
    type From = T;
    const READ: bool = true;
    const WRITE: bool = true;
    fn new(value: &mut T) -> &mut T {
        value
    }
}
impl<T: Reflect> AccessField for Set<'_, T> {
    type Concrete<'z> = Set<'z, T>;
    type From = T;
    const READ: bool = false;
    const WRITE: bool = true;
    fn new(value: &mut T) -> Set<T> {
        Set(value)
    }
}

// -----
//
// impl Access
//
// -----

impl<T0> Access for T0
where
    T0: AccessField,
{
    type Concrete<'z> = T0::Concrete<'z>;
    type Paths = &'static str;
    type ParsedPaths = ParsedPath;

    fn record(paths: &Self::Paths, rec: &mut CompAccessRecorder) {
        if T0::READ {
            rec.read(paths);
        }
        if T0::WRITE {
            rec.write(paths);
        }
    }
    fn parse_path(paths: &'static str) -> ParsedPath {
        ParsedPath::parse(paths).unwrap() // TODO(err)
    }
    fn extract<'z, C: Component + Reflect>(
        value: Mut<'z, C>,
        path: &mut Self::ParsedPaths,
    ) -> Self::Concrete<'z> {
        T0::new(path.element_mut(value.into_inner()).unwrap())
    }
}

impl<T0, T1, T2> Access for (T0, T1, T2)
where
    T0: AccessField,
    T1: AccessField,
    T2: AccessField,
{
    type Concrete<'a> = (T0::Concrete<'a>, T1::Concrete<'a>, T2::Concrete<'a>);
    type Paths = [&'static str; 3];
    type ParsedPaths = [ParsedPath; 3];

    fn record(paths: &Self::Paths, rec: &mut CompAccessRecorder) {
        if T0::READ {
            rec.read(paths[0]);
        }
        if T0::WRITE {
            rec.write(paths[0]);
        }
        if T1::READ {
            rec.read(paths[1]);
        }
        if T1::WRITE {
            rec.write(paths[1]);
        }
        if T2::READ {
            rec.read(paths[2]);
        }
        if T2::WRITE {
            rec.write(paths[2]);
        }
    }
    fn parse_path(paths: Self::Paths) -> Self::ParsedPaths {
        paths.map(|p| ParsedPath::parse(p).unwrap())
    }
    fn extract<'k, C: Component + Reflect>(
        value: Mut<'k, C>,
        path: &mut Self::ParsedPaths,
    ) -> Self::Concrete<'k> {
        let multi = Multipath::new(path);
        let (p0, p1, p2) = multi.split(value.into_inner()).unwrap();
        (T0::new(p0), T1::new(p1), T2::new(p2))
    }
}

// -----
//
// Item & Set
//
// -----

// Write only access to a mutable value
pub struct Set<'a, T>(&'a mut T);

impl<'a, T> Set<'a, T> {
    /// Set content to value.
    pub fn set(&mut self, value: T) {
        *self.0 = value
    }
    /// Get a `&mut` to the underlying value.
    ///
    /// This may be preferred over [`Self::set`] **for performance.**
    ///
    /// Typically, if `T` is a `Vec`, you don't want to allocate a new `Vec`
    /// and rewrite its content. Instead, you want to `vec.clear()` it and
    /// then extend it — re-using already-allocated memory.
    ///
    /// # Limitations
    ///
    /// No trace of the original value must remain. The original value should
    /// be fully replaced.
    ///
    /// For example, if you `vec.push(value)`, the original content is part
    /// of the value after it has been set.
    ///
    /// This has no soundness issue, but dependency resolution will be broken
    /// otherwise.
    pub fn write_only_ref(self) -> &'a mut T {
        self.0
    }
}

pub struct Item<C, A>(pub A, pub PhantomData<fn(C)>);

impl<C: Component + Reflect, A: Access> Item<C, A> {
    pub fn record<'a: 'c, 'b: 'c, 'c>(paths: &A::Paths, build: &'c mut FnAccessRecorder<'a, 'b>) {
        let mut rec = build.for_component::<C>();
        A::record(paths, &mut rec)
    }
}
pub fn item_from_query<'a, C: Component + Reflect, A: Access>(
    query: Mut<'a, C>,
    paths: &mut A::ParsedPaths,
) -> Item<C, A::Concrete<'a>> {
    Item(A::extract(query, paths), PhantomData)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    fn item_single<'a, C: Component + Reflect, A: Access<Concrete<'a> = A>>(
        world: &'a mut World,
        paths: A::Paths,
    ) -> Item<C, A> {
        let mut parsed = A::parse_path(paths);
        let mut query = world.query::<&mut C>();
        let entity = query.single_mut(world);
        item_from_query::<C, A>(entity, &mut parsed)
    }

    #[test]
    fn item_parses() {
        let mut world = World::new();

        let orig_rot = Quat::from_euler(EulerRot::XYZ, 4.0, 5.0, 6.0);
        let orig_scale = Vec3::new(7.0, 8.0, 9.0);

        world.spawn(Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: orig_rot,
            scale: orig_scale,
        });
        let paths = [".translation.x", ".scale", ".rotation"];
        let Item((mut x, scale, rot), ..) =
            item_single::<Transform, (Set<f32>, &mut Vec3, &Quat)>(&mut world, paths);

        x.set(10.0);
        *scale *= 3.0;
        assert_eq!(rot, &orig_rot);

        let transform = world.query::<&Transform>().single(&world);
        assert_eq!(transform.translation.x, 10.0);
        assert_eq!(transform.scale, orig_scale * 3.0);
    }
}
