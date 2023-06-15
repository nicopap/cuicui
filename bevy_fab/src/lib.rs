#![allow(clippy::new_without_default)]
//! Integrate the [`fab`] crate with bevy.

pub mod fmt_system;
mod local;
mod make;
mod track;
pub mod trait_extensions;
mod world;

use std::{fmt::Arguments, marker::PhantomData};

use bevy::app::{App, CoreSet, Plugin};
use bevy::ecs::prelude::*;
use bevy::ecs::query::WorldQuery;
use bevy::ecs::system::{EntityCommands, StaticSystemParam, SystemParam, SystemParamItem};
use bevy::prelude::Children;
use fab::modify::{FieldsOf, Indexed};
use fab_parse::Parsable;
use reflect_query::predefined::QueryablePlugin;

pub use fmt_system::{FmtSystem, IntoFmtSystem};
pub use local::LocalBindings;
pub use make::{parse_into_resolver_system, ParseFormatString};
pub use reflect_query::ReflectQueryable;
pub use track::UserFmt;
pub use world::{update_hooked, Hook, StyleFn, Styles, WorldBindings};

// omg please don't look at this, I swear this is temporary
/// A [`fab::Modify`] that works on a bevy component and can be inserted in the ECS.
pub trait BevyModify: Send + Sync + 'static
where
    Self: for<'a, 'w, 's> Parsable<
        Items<'a, 'w, 's> = Items<'a, 'w, 's, Self::Wq>,
        Item<'a> = <Self::Wq as WorldQuery>::Item<'a>,
    >,
{
    type Wq: WorldQuery;
    type Param: SystemParam;
    type ItemsCtorData: Send + Sync;

    fn set_content(&mut self, s: Arguments);
    fn init_content(s: Arguments) -> Self;

    fn context<'a>(param: &'a SystemParamItem<Self::Param>) -> Self::Context<'a>;
    fn spawn_items(
        extra: &Self::ItemsCtorData,
        items: Vec<Self::MakeItem>,
        cmds: &mut EntityCommands,
    );
}

#[derive(SystemParam)]
pub struct Param<'w, 's, Ctx: SystemParam + 'static, It: WorldQuery + 'static> {
    context: StaticSystemParam<'w, 's, Ctx>,
    query: Query<'w, 's, It>,
}
pub struct Items<'a, 'w, 's, It: WorldQuery> {
    children: Option<&'a Children>,
    query: Query<'w, 's, It>,
}
impl<'a, 'w, 's, Wq, M> Indexed<M> for Items<'a, 'w, 's, Wq>
where
    Wq: WorldQuery,
    M: BevyModify<Wq = Wq>,
{
    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<M::Item<'_>> {
        let &entity = self.children?.get(index)?;
        self.query.get_mut(entity).ok()
    }
}

pub fn update_items_system<BM: BevyModify>(
    mut query: Query<(&mut LocalBindings<BM>, &Children)>,
    mut world_bindings: ResMut<WorldBindings<BM>>,
    params: StaticSystemParam<Param<BM::Param, BM::Wq>>,
) where
    FieldsOf<BM>: Sync + Send,
{
    let Param { context, query: param_query } = params.into_inner();
    let mut items = Items { children: None, query: param_query };
    for (mut local_data, children) in &mut query {
        let context = &BM::context(&context);
        items.children = Some(children);
        local_data.update(&mut items, &world_bindings, context);
    }
    world_bindings.bindings.reset_changes();
}

/// Manages [`BevyModify`] living in the ECS as [`LocalBindings`] and a global
/// [`WorldBindings`]. Also [`Hook`]s to automatically update reflection-based
/// bindings.
pub struct FabPlugin<BM: BevyModify>(PhantomData<fn(BM)>);
impl<BM: BevyModify> FabPlugin<BM>
where
    FieldsOf<BM>: Sync + Send,
{
    pub fn new() -> Self {
        FabPlugin(PhantomData)
    }
}
impl<BM: BevyModify> Plugin for FabPlugin<BM>
where
    for<'b> &'b mut BM::MakeItem: Into<BM::Item<'b>>,
    FieldsOf<BM>: Sync + Send,
{
    fn build(&self, app: &mut App) {
        use CoreSet::PostUpdate;
        app.add_plugin(QueryablePlugin)
            .init_resource::<WorldBindings<BM>>()
            .init_resource::<Styles<BM>>()
            .add_system(update_hooked::<BM>.in_base_set(PostUpdate))
            .add_system(update_items_system::<BM>.in_base_set(PostUpdate))
            .add_system(parse_into_resolver_system::<BM>);
    }
}
