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
use fab::prefab::FieldsOf;
use fab_parse::{Parsable, TransformedTree};

use track::Hooks;

pub use local::PrefabLocal;
pub use make::{parse_into_resolver_system, ParseFormatString};
pub use track::{update_component_trackers_system, update_hooked, TrackerBundle};
pub use world::PrefabWorld;

pub trait BevyModify: Parsable + fmt::Write + From<String> + Send + Sync + 'static {
    type Param: SystemParam;
    type ItemsCtorData: Send + Sync;

    fn context<'a>(param: &'a SystemParamItem<Self::Param>) -> Self::Context<'a>;

    fn make_items(extra: &Self::ItemsCtorData, items: Vec<Self::Item>) -> Self::Items;

    fn transform(tree: TransformedTree<'_, Self>) -> TransformedTree<'_, Self> {
        tree
    }
}

pub fn update_items_system<BM: BevyModify, const R: usize>(
    mut query: Query<(&mut PrefabLocal<BM, R>, &mut BM::Items)>,
    mut world_bindings: ResMut<PrefabWorld<BM>>,
    params: StaticSystemParam<BM::Param>,
) where
    BM::Items: Component,
    FieldsOf<BM>: Sync + Send,
{
    let context = BM::context(&params);
    for (mut local_data, mut items) in &mut query {
        local_data.update(&mut items, &world_bindings, &context);
    }
    world_bindings.0.reset_changes();
}

/// Manage a `Prefab` and [`Hooks`] to update the prefab's item as a component
/// in the bevy ECS.
pub struct FabPlugin<BM: BevyModify, const R: usize>(PhantomData<fn(BM)>);
impl<BM: BevyModify, const R: usize> FabPlugin<BM, R>
where
    BM::Items: Component,
    FieldsOf<BM>: Sync + Send,
{
    pub fn new() -> Self {
        FabPlugin(PhantomData)
    }
}
impl<BM: BevyModify, const R: usize> Plugin for FabPlugin<BM, R>
where
    BM::Items: Component,
    FieldsOf<BM>: Sync + Send,
{
    fn build(&self, app: &mut App) {
        use CoreSet::PostUpdate;
        app.init_resource::<PrefabWorld<BM>>()
            .init_resource::<Hooks<BM>>()
            .add_system(update_hooked::<BM>.in_base_set(PostUpdate))
            .add_system(update_items_system::<BM, R>.in_base_set(PostUpdate))
            .add_system(update_component_trackers_system::<BM>.in_base_set(PostUpdate))
            .add_system(parse_into_resolver_system::<BM, R>);
    }
}
