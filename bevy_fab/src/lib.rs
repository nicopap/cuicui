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

pub trait MakeMut<'a, I: 'a> {
    fn make_mut(self) -> I;
}
impl<'a, T1, T2> MakeMut<'a, (&'a mut T1, &'a mut T2)> for (Mut<'a, T1>, Mut<'a, T2>) {
    fn make_mut(self) -> (&'a mut T1, &'a mut T2) {
        (self.0.into_inner(), self.1.into_inner())
    }
}

// omg please don't look at this, I swear this is temporary
/// A [`fab::Modify`] that works on a bevy component and can be inserted in the ECS.
pub trait BevyModify: Parsable + Send + Sync + 'static {
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
    fn add_update_system(app: &mut App);
}

pub struct Items<'a, 'w, 's, It: WorldQuery> {
    children: Option<&'a Children>,
    query: Query<'w, 's, It>,
}

impl<'a, 'w, 's, It: WorldQuery> Items<'a, 'w, 's, It> {
    pub fn new(children: Option<&'a Children>, query: Query<'w, 's, It>) -> Self {
        Items { children, query }
    }
}
impl<'a, 'w, 's, Wq, M> Indexed<M> for Items<'a, 'w, 's, Wq>
where
    Wq: WorldQuery,
    M: BevyModify,
    for<'b> Wq::Item<'b>: MakeMut<'b, M::Item<'b>>,
{
    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<M::Item<'_>> {
        let &entity = self.children?.get(index)?;
        Some(self.query.get_mut(entity).ok()?.make_mut())
    }
}

pub fn update_children_system<Wq: WorldQuery, BM: BevyModify>(
    mut query: Query<(&mut LocalBindings<BM>, Option<&Children>)>,
    mut world_bindings: ResMut<WorldBindings<BM>>,
    ctx_params: StaticSystemParam<BM::Param>,
    items_query: Query<Wq>,
) where
    BM: for<'a, 'w, 's> Parsable<Items<'a, 'w, 's> = Items<'a, 'w, 's, Wq>>,
    for<'b> Wq::Item<'b>: MakeMut<'b, BM::Item<'b>>,
    FieldsOf<BM>: Sync + Send,
{
    let context = BM::context(&ctx_params);
    let mut items = Items { children: None, query: items_query };
    for (mut local_data, children) in &mut query {
        items.children = children;
        local_data.update(&mut items, &world_bindings, &context);
    }
    world_bindings.bindings.reset_changes();
}

pub fn update_component_items<BM: BevyModify>(
    mut query: Query<(&mut LocalBindings<BM>, &mut BM::Items<'_, '_, '_>)>,
    mut world_bindings: ResMut<WorldBindings<BM>>,
    params: StaticSystemParam<BM::Param>,
) where
    for<'a, 'b, 'c> BM::Items<'a, 'b, 'c>: Component,
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
    FieldsOf<BM>: Sync + Send,
{
    pub fn new() -> Self {
        FabPlugin(PhantomData)
    }
}
impl<BM: BevyModify> Plugin for FabPlugin<BM>
where
    FieldsOf<BM>: Sync + Send,
{
    fn build(&self, app: &mut App) {
        use CoreSet::PostUpdate;
        app.add_plugin(QueryablePlugin)
            .init_resource::<WorldBindings<BM>>()
            .init_resource::<Styles<BM>>()
            .add_system(update_hooked::<BM>.in_base_set(PostUpdate))
            .add_system(parse_into_resolver_system::<BM>);
        BM::add_update_system(app);
    }
}
