//! Fetch data from [`Reflect`] [`Component`]s and [`Resource`]s to directly
//! update rich text contexts.

use bevy::{prelude::*, reflect::GetPath};

use crate::richtext::ModifyBox;

use super::IntoModify;

pub trait Fetch {
    fn fetch(&self, world: &World) -> Option<ModifyBox>;
}
pub struct DynamicFetcher<F: Fn(&World) -> Option<ModifyBox>>(pub F);
impl<F: Fn(&World) -> Option<ModifyBox>> DynamicFetcher<F> {
    // TODO(feat): useful ways to construct a DynamicFetcher
}

impl<F: Fn(&World) -> Option<ModifyBox>> Fetch for DynamicFetcher<F> {
    fn fetch(&self, world: &World) -> Option<ModifyBox> {
        (self.0)(world)
    }
}

pub struct ComponentFetcher {
    entity: Entity,
    fetcher: fn(&World, Entity) -> Option<&dyn Reflect>,
    reader: fn(&dyn Reflect, &str) -> Option<ModifyBox>,
    reflect_path: &'static str,
}
impl ComponentFetcher {
    pub fn new<C, F>(entity: Entity, reflect_path: &'static str) -> Self
    where
        C: Reflect + Component,
        F: Reflect + IntoModify + Clone,
    {
        ComponentFetcher {
            fetcher: |w, e| w.get::<C>(e).map::<&dyn Reflect, _>(|m| m),
            reader: |reflect, path| Some(reflect.path::<F>(path).ok()?.clone().into_modify()),
            reflect_path,
            entity,
        }
    }
}
impl Fetch for ComponentFetcher {
    fn fetch(&self, world: &World) -> Option<ModifyBox> {
        let component = (self.fetcher)(world, self.entity)?;
        (self.reader)(component, self.reflect_path)
    }
}

pub struct ResourceFetcher {
    fetcher: fn(&World) -> Option<&dyn Reflect>,
    reader: fn(&dyn Reflect, &str) -> Option<ModifyBox>,
    reflect_path: &'static str,
}
impl ResourceFetcher {
    /// Create a fetcher for a resource.
    ///
    /// ## Type parameters
    ///
    /// - `R`: The type of the resource to query for.
    /// - `F`: The type of the field of `R` designated with `reflect_path`.
    pub fn new<R, F>(reflect_path: &'static str) -> Self
    where
        R: Reflect + Resource,
        F: Reflect + IntoModify + Clone,
    {
        ResourceFetcher {
            fetcher: |w| w.get_resource::<R>().map::<&dyn Reflect, _>(|m| m),
            reader: |reflect, path| Some(reflect.path::<F>(path).ok()?.clone().into_modify()),
            reflect_path,
        }
    }
}
impl Fetch for ResourceFetcher {
    fn fetch(&self, world: &World) -> Option<ModifyBox> {
        let resource = (self.fetcher)(world)?;
        (self.reader)(resource, self.reflect_path)
    }
}
