use std::marker::PhantomData;

use bevy::{
    ecs::system::{EntityCommands, SystemParamItem},
    prelude::*,
};
use bevy_mod_sysfail::quick_sysfail;
use bevy_ui_navigation::prelude::*;

use crate::{Prefab, Widge, WorldValue};

#[derive(Component)]
pub struct ActionButton {
    pub action: Box<dyn System<In = (Vec<Entity>, NavRequest), Out = ()>>,
}
impl ActionButton {
    pub fn new<P, F: IntoSystem<(Vec<Entity>, NavRequest), (), P>>(action: F) -> Self {
        ActionButton { action: Box::new(IntoSystem::into_system(action)) }
    }
}
pub struct ActionButtonPrefab<P, S: IntoSystem<(Vec<Entity>, NavRequest), (), P> + Clone> {
    pub action: S,
    _params: PhantomData<fn(P)>,
}
impl<P, S: IntoSystem<(Vec<Entity>, NavRequest), (), P> + Clone> Prefab
    for ActionButtonPrefab<P, S>
{
    type Param = ();

    fn spawn(&self, mut commands: EntityCommands, _: &mut ()) {
        commands.insert(ActionButton::new(self.action.clone()));
    }
}
impl<P, S: IntoSystem<(Vec<Entity>, NavRequest), (), P> + Clone> WorldValue
    for ActionButtonPrefab<P, S>
{
    type Value = ();

    type ReadParam<'w, 's> = ();

    fn read(_: Entity, _: &SystemParamItem<Self::ReadParam<'_, '_>>) -> Option<Self::Value> {
        Some(())
    }
}
impl<P, S: IntoSystem<(Vec<Entity>, NavRequest), (), P> + Clone> Widge
    for ActionButtonPrefab<P, S>
{
}

#[quick_sysfail]
fn check_action_button_valid(
    world: &World,
    buttons: Query<(&ActionButton, Entity, Option<&Name>), Changed<ActionButton>>,
) {
    let action_button_id = world.component_id::<ActionButton>()?;
    for (button, entity, maybe_name) in &buttons {
        let access = button.action.component_access();
        if access.has_read(action_button_id) {
            let name = maybe_name.map_or(format!("{entity:?}"), |n| n.to_string());
            panic!(
                "{name}'s ActionButton's system accesses the ActionButton component! \
                It shouldn't, otherwise this would cause conflicting access. \
                Please do not query for ActionButton in actions"
            );
        }
    }
}
fn activate_button(
    world: &World,
    mut actions: Query<&mut ActionButton>,
    mut nav_events: EventReader<NavEvent>,
) {
    for nav_event in nav_events.iter() {
        if let NavEvent::NoChanges { from, request } = nav_event {
            let Ok(mut button) = actions.get_mut(*from.first()) else { continue; };
            let input = (from.to_vec(), *request);
            // SAFETY:
            // - This is an exclusive system, therefore the only concurrent
            //   access we have to worry about is &mut ActionButton.
            // - We check in check_action_button_valid that the system doesn't
            //   access in any way ActionButton.
            // Ok, this is unsafe, because it's not guarenteed, you need to
            // make sure the only place activate_button is used, it is added
            // with a `at_end()` descriptor.
            unsafe {
                button.action.run_unsafe(input, world);
            }
        }
    }
}
pub struct Plug;
impl Plugin for Plug {
    fn build(&self, app: &mut App) {
        app.add_system(check_action_button_valid)
            .add_system(activate_button.at_end().after(check_action_button_valid));
    }
}
