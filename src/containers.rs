use bevy::{
    ecs::system::{EntityCommands, SystemParamItem},
    prelude::{BuildChildren, Children, Component, Entity, Query, With},
};

use crate::{Prefab, ReadWorldValue, UiControl};

#[derive(Component)]
pub struct ListItem;

pub struct List<T: UiControl> {
    items: Vec<T>,
}
impl<T: UiControl> ReadWorldValue for List<T> {
    type Value = Vec<T::Value>;
    type ReadParam<'w, 's> = (
        Query<'w, 's, Entity, With<ListItem>>,
        Query<'w, 's, &'static Children>,
        T::ReadParam<'w, 's>,
    );

    fn read(
        entity: Entity,
        (items, children, param): &SystemParamItem<Self::ReadParam<'_, '_>>,
    ) -> Option<Self::Value> {
        items
            .iter_many(children.get(entity).ok()?)
            .map(|item| T::read(item, param))
            .collect()
    }
}
impl<T: UiControl> Prefab for List<T> {
    type Param = T::Param;

    fn spawn(&self, mut commands: EntityCommands, param: &mut SystemParamItem<Self::Param>) {
        commands.with_children(|commands| {
            for elem in &self.items {
                let commands = commands.spawn_empty();
                elem.spawn(commands, param);
            }
        });
    }
}
impl<T: UiControl> UiControl for List<T> {}
