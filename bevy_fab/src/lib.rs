#![allow(clippy::new_without_default)]
//! Integrate the [`fab`] crate with bevy.

mod local;
mod make;
mod track;
mod world;

use std::{fmt, marker::PhantomData};

use bevy::app::{App, CoreSet, Plugin};
use bevy::ecs::prelude::*;
use bevy::ecs::system::{StaticSystemParam, SystemParam, SystemParamItem};
use fab::prefab::{FieldsOf, PrefabContext};
use fab_parse::{ParsablePrefab, TransformedTree};

use track::Hooks;

pub use local::PrefabLocal;
pub use make::{parse_into_resolver_system, ParseFormatString};
pub use track::{update_component_trackers_system, update_hooked, TrackerBundle};
pub use world::PrefabWorld;

pub trait BevyPrefab: ParsablePrefab {
    type Param: SystemParam;
    type ItemsCtorData: Send + Sync;

    fn context<'a>(param: &'a SystemParamItem<Self::Param>) -> PrefabContext<'a, Self>;

    fn make_items(extra: &Self::ItemsCtorData, items: Vec<Self::Item>) -> Self::Items;

    fn transform(tree: TransformedTree<'_, Self>) -> TransformedTree<'_, Self> {
        tree
    }
}

pub fn update_items_system<BP: BevyPrefab + 'static, const R: usize>(
    mut query: Query<(&mut PrefabLocal<BP, R>, &mut BP::Items)>,
    mut world_bindings: ResMut<PrefabWorld<BP>>,
    params: StaticSystemParam<BP::Param>,
) where
    BP::Items: Component,
    BP::Modify: fmt::Write + From<String>,
    FieldsOf<BP>: Sync + Send,
{
    let context = BP::context(&params);
    for (mut local_data, mut items) in &mut query {
        local_data.update(&mut items, &world_bindings, &context);
    }
    world_bindings.0.reset_changes();
}

/// Manage a `Prefab` and [`Hooks`] to update the prefab's item as a component
/// in the bevy ECS.
pub struct FabPlugin<BP: BevyPrefab + 'static, const R: usize>(PhantomData<fn(BP)>);
impl<BP: BevyPrefab, const R: usize> FabPlugin<BP, R>
where
    BP::Items: Component,
    BP::Modify: fmt::Write + From<String>,
    FieldsOf<BP>: Sync + Send,
{
    pub fn new() -> Self {
        FabPlugin(PhantomData)
    }
}
impl<BP: BevyPrefab + 'static, const R: usize> Plugin for FabPlugin<BP, R>
where
    BP::Items: Component,
    BP::Modify: fmt::Write + From<String>,
    FieldsOf<BP>: Sync + Send,
{
    fn build(&self, app: &mut App) {
        use CoreSet::PostUpdate;
        app.init_resource::<PrefabWorld<BP>>()
            .init_resource::<Hooks<BP::Modify>>()
            .add_system(update_hooked::<BP>.in_base_set(PostUpdate))
            .add_system(update_items_system::<BP, R>.in_base_set(PostUpdate))
            .add_system(update_component_trackers_system::<BP>.in_base_set(PostUpdate))
            .add_system(parse_into_resolver_system::<BP, R>);
    }
}
