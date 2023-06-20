use std::marker::PhantomData;

use bevy::prelude::{Component, Mut, World};
use bevy::reflect::{ParsedPath, Reflect};

use crate::access_registry::CompAccessRecorder;
use crate::access_registry::FnAccessRecorder;
use crate::split_reflect_path::Multipath;

pub type Paths<A> = <A as Access<'static>>::Paths;
pub type ParsedPaths<A> = <A as Access<'static>>::ParsedPaths;

trait AccessField<'a> {
    type From: Reflect;
    const READ: bool;
    const WRITE: bool;
    fn new(value: &'a mut Self::From) -> Self;
}

pub trait Access<'z> {
    type Paths: Send + Sync + 'static;
    type ParsedPaths: Send + Sync + 'static;

    fn record(paths: Self::Paths, rec: &mut CompAccessRecorder);
    fn parse_path(paths: Self::Paths) -> Self::ParsedPaths;
    fn extract<C: Component + Reflect>(value: Mut<'z, C>, path: &mut Self::ParsedPaths) -> Self;
}

// -----
//
// impl AccessField
//
// -----

impl<'a, T: Reflect + 'a> AccessField<'a> for &'a T {
    type From = T;
    const READ: bool = true;
    const WRITE: bool = false;
    fn new(value: &'a mut T) -> Self {
        value
    }
}
impl<'a, T: Reflect + 'a> AccessField<'a> for &'a mut T {
    type From = T;
    const READ: bool = true;
    const WRITE: bool = true;
    fn new(value: &'a mut T) -> Self {
        value
    }
}
impl<'a, T: Reflect + 'a> AccessField<'a> for Set<'a, T> {
    type From = T;
    const READ: bool = false;
    const WRITE: bool = true;
    fn new(value: &'a mut T) -> Self {
        Set(value)
    }
}

// -----
//
// impl Access
//
// -----

impl<'z, T0> Access<'z> for T0
where
    T0: AccessField<'z>,
{
    type Paths = &'static str;
    type ParsedPaths = ParsedPath;

    fn record(paths: Self::Paths, rec: &mut CompAccessRecorder) {
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
    fn extract<C: Component + Reflect>(value: Mut<'z, C>, path: &mut Self::ParsedPaths) -> Self {
        T0::new(path.element_mut(value.into_inner()).unwrap())
    }
}

impl<'z, T0, T1, T2> Access<'z> for (T0, T1, T2)
where
    T0: AccessField<'z>,
    T1: AccessField<'z>,
    T2: AccessField<'z>,
{
    type Paths = [&'static str; 3];
    type ParsedPaths = [ParsedPath; 3];

    fn record(paths: Self::Paths, rec: &mut CompAccessRecorder) {
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
    fn extract<C: Component + Reflect>(value: Mut<'z, C>, path: &mut Self::ParsedPaths) -> Self {
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

pub struct Item<C, A>(pub A, PhantomData<fn(C)>);

impl<C: Component + Reflect, A> Item<C, A> {
    pub fn record<'a>(paths: A::Paths, build: &mut FnAccessRecorder)
    where
        A: Access<'a>,
    {
        let mut rec = build.for_component::<C>();
        A::record(paths, &mut rec)
    }
    pub fn from_query<'a>(query: Mut<'a, C>, paths: &mut A::ParsedPaths) -> Self
    where
        A: Access<'a>,
    {
        Item(A::extract(query, paths), PhantomData)
    }
    fn single<'a>(world: &'a mut World, paths: A::Paths) -> Self
    where
        A: Access<'a>,
    {
        let mut parsed = <A as Access<'a>>::parse_path(paths);
        let mut query = world.query::<&mut C>();
        let entity = query.single_mut(world);
        Self::from_query(entity, &mut parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

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
            Item::<Transform, (Set<f32>, &mut Vec3, &Quat)>::single(&mut world, paths);

        x.set(10.0);
        *scale *= 3.0;
        assert_eq!(rot, &orig_rot);

        let transform = world.query::<&Transform>().single(&world);
        assert_eq!(transform.translation.x, 10.0);
        assert_eq!(transform.scale, orig_scale * 3.0);
    }
}
