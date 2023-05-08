use bevy::{
    ecs::system::{EntityCommands, SystemParamItem},
    prelude::*,
};
use bevy_ui_navigation::prelude::*;

use crate::{widge, ExtractPrefab, Prefab, Widge, WorldValue};

#[derive(Debug)]
pub struct Check {
    pub checked: bool,
    pub frame: widge::Image,
    pub fill: widge::Image,
}
#[derive(Component, Clone, Copy)]
struct CheckCompo {
    checked: bool,
}
impl Prefab for Check {
    type Param = ();
    fn spawn(&self, mut commands: EntityCommands, _: &mut SystemParamItem<Self::Param>) {
        commands
            .insert(CheckCompo { checked: self.checked })
            .with_children(|cmds| {});
    }
}
impl ExtractPrefab for Check {
    type ExtractParam<'w, 's> = Query<'w, 's, &'static CheckCompo>;
    fn extract(
        entity: Entity,
        params: &SystemParamItem<Self::ExtractParam<'_, '_>>,
    ) -> Option<Self> {
        params.get(entity).ok().copied()
    }
}
impl WorldValue for Check {
    type Value = bool;
    type ReadParam<'w, 's> = Query<'w, 's, &'static CheckCompo>;
    fn read(
        entity: Entity,
        params: &SystemParamItem<Self::ReadParam<'_, '_>>,
    ) -> Option<Self::Value> {
        params.get(entity).ok().map(|c| c.checked)
    }
}
impl Widge for Check {}

fn activate(mut checkboxes: Query<&mut CheckCompo>, mut nav_events: EventReader<NavEvent>) {
    nav_events
        .nav_iter()
        .activated_in_query_foreach_mut(&mut checkboxes, |mut check| {
            check.checked = !check.checked;
        });
}
pub struct Plug;
impl Plugin for Plug {
    fn build(&self, app: &mut App) {
        app.add_system(activate);
    }
}
