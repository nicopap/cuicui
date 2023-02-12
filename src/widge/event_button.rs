use std::marker::PhantomData;

use bevy::{
    ecs::{
        event::Event,
        system::{EntityCommands, SystemParamItem},
    },
    prelude::*,
};
use bevy_ui_navigation::prelude::*;

use crate::{ExtractPrefab, Prefab, Widge, WorldValue};

pub trait PodEvent: Into<u32> + From<u32> + Copy {}
#[derive(Component)]
pub struct Button {
    event: u32,
}
impl Button {
    pub fn new<T: PodEvent>(event: T) -> Self {
        Button { event: event.into() }
    }
}
pub struct ButtonPrefab<T: PodEvent> {
    event: T,
}
impl<T: PodEvent> Prefab for ButtonPrefab<T> {
    type Param = ();
    fn spawn(&self, mut commands: EntityCommands, _: &mut SystemParamItem<Self::Param>) {
        commands.insert(Button::new(self.event));
    }
}
impl<T: PodEvent> ExtractPrefab for ButtonPrefab<T> {
    type ExtractParam<'w, 's> = Query<'w, 's, &'static Button>;
    fn extract(
        entity: Entity,
        params: &SystemParamItem<Self::ExtractParam<'_, '_>>,
    ) -> Option<Self> {
        Some(ButtonPrefab { event: params.get(entity).ok()?.event.into() })
    }
}
impl<T: PodEvent> WorldValue for ButtonPrefab<T> {
    type Value = ();
    type ReadParam<'w, 's> = ();
    fn read(_: Entity, _: &SystemParamItem<Self::ReadParam<'_, '_>>) -> Option<Self::Value> {
        Some(())
    }
}
impl<T: PodEvent> Widge for ButtonPrefab<T> {}

fn activate_button<T: Event + PodEvent>(
    actions: Query<&Button>,
    mut nav_events: EventReader<NavEvent>,
    mut button_events: EventWriter<T>,
) {
    for action in nav_events.nav_iter().activated_in_query(&actions) {
        button_events.send(action.event.into());
    }
}
pub struct Plug<T: Event + PodEvent>(PhantomData<fn(T)>);
impl<T: Event + PodEvent> Plug<T> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}
impl<T: Event + PodEvent> Plugin for Plug<T> {
    fn build(&self, app: &mut App) {
        app.add_event::<T>().add_system(activate_button::<T>);
    }
}
