#![allow(clippy::new_without_default)]
//! Integrate the [`fab`] crate with bevy.

mod local;
mod make;
mod track;
pub mod trait_extensions;
mod world;

use std::{fmt::Arguments, marker::PhantomData};

use bevy::app::{App, CoreSet, Plugin};
use bevy::ecs::prelude::*;
use bevy::ecs::system::{StaticSystemParam, SystemParam, SystemParamItem};
use fab::modify::FieldsOf;
use fab_parse::Parsable;
use reflect_query::BaseReflectQueryablePlugin;

pub use local::LocalBindings;
pub use make::{parse_into_resolver_system, ParseFormatString};
pub use reflect_query::ReflectQueryable;
pub use world::{update_hooked, Hook, StyleFn, Styles, WorldBindings};

/// A [`fab::Modify`] that works on a bevy component and can be inserted in the ECS.
pub trait BevyModify: Parsable + Send + Sync + 'static {
    type Param: SystemParam;
    type ItemsCtorData: Send + Sync;

    fn set_content(&mut self, s: Arguments);
    fn init_content(s: Arguments) -> Self;

    fn context<'a>(param: &'a SystemParamItem<Self::Param>) -> Self::Context<'a>;

    fn make_items(extra: &Self::ItemsCtorData, items: Vec<Self::Item>) -> Self::Items;
}

pub fn update_items_system<BM: BevyModify>(
    mut query: Query<(&mut LocalBindings<BM>, &mut BM::Items)>,
    mut world_bindings: ResMut<WorldBindings<BM>>,
    params: StaticSystemParam<BM::Param>,
) where
    BM::Items: Component,
    FieldsOf<BM>: Sync + Send,
{
    let context = BM::context(&params);
    for (mut local_data, mut items) in &mut query {
        local_data.update(&mut items, &world_bindings, &context);
    }
    world_bindings.bindings.reset_changes();
}

/// Manages [`BevyModify`] living in the ECS as [`LocalBindings`] and a global
/// [`WorldBindings`]. Also [`Hook`]s to automatically update reflection-based
/// bindings.
pub struct FabPlugin<BM: BevyModify>(PhantomData<fn(BM)>);
impl<BM: BevyModify> FabPlugin<BM>
where
    BM::Items: Component,
    FieldsOf<BM>: Sync + Send,
{
    pub fn new() -> Self {
        FabPlugin(PhantomData)
    }
}
impl<BM: BevyModify> Plugin for FabPlugin<BM>
where
    BM::Items: Component,
    FieldsOf<BM>: Sync + Send,
{
    fn build(&self, app: &mut App) {
        use CoreSet::PostUpdate;
        app.add_plugin(BaseReflectQueryablePlugin)
            .init_resource::<WorldBindings<BM>>()
            .init_resource::<Styles<BM>>()
            .add_system(update_hooked::<BM>.in_base_set(PostUpdate))
            .add_system(update_items_system::<BM>.in_base_set(PostUpdate))
            .add_system(parse_into_resolver_system::<BM>);
    }
}
