use bevy::{
    ecs::system::{EntityCommands, SystemParamItem},
    prelude::{BuildChildren, Children, Component, Entity, In, Query, With},
};

use crate::Widge;

#[derive(Component)]
pub struct ListItem;

pub struct List<T: Widge> {
    items: Vec<T>,
}
impl<T: Widge> Widge for List<T> {
    fn spawn(&self, mut commands: EntityCommands) {
        commands.with_children(|commands| {
            for elem in &self.items {
                let commands = commands.spawn(ListItem);
                elem.spawn(commands);
            }
        });
    }

    type ReadSystemParam<'w, 's> = (
        Query<'w, 's, Entity, With<ListItem>>,
        Query<'w, 's, &'static Children>,
        T::ReadSystemParam<'w, 's>,
    );
    fn read_from_ecs(
        entity: bevy::prelude::In<Entity>,
        (items, children, param): &SystemParamItem<Self::ReadSystemParam<'_, '_>>,
    ) -> Option<Self>
    where
        Self: Sized,
    {
        let items = items
            .iter_many(children.get(entity.0).ok()?)
            .map(|item| T::read_from_ecs(In(item), param))
            .collect::<Option<_>>()?;
        Some(List { items })
    }
}
