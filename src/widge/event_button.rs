use std::marker::PhantomData;

use bevy::{
    ecs::{
        event::Event,
        system::{EntityCommands, SystemParamItem},
    },
    prelude::*,
};
use bevy_ui_navigation::prelude::*;

use crate::Widge;

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

pub struct ButtonWidge<T: PodEvent> {
    event: T,
}
impl<T: PodEvent> Widge for ButtonWidge<T> {
    fn spawn(&self, mut commands: EntityCommands) {
        commands.insert(Button::new(self.event));
    }

    type ReadSystemParam<'w, 's> = Query<'w, 's, &'static Button>;
    fn read_from_ecs(
        entity: In<Entity>,
        params: &SystemParamItem<Self::ReadSystemParam<'_, '_>>,
    ) -> Option<Self>
    where
        Self: Sized,
    {
        Some(ButtonWidge { event: params.get(entity.0).ok()?.event.into() })
    }
}

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
