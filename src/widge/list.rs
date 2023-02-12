use bevy::{
    ecs::system::{EntityCommands, SystemParamItem},
    prelude::{BuildChildren, Children, Component, Entity, Query, With},
};

use crate::{ExtractPrefab, Prefab, Widge, WorldValue};

#[derive(Component)]
pub struct ListItem;

pub struct List<T: Widge> {
    items: Vec<T>,
}
impl<T: Widge> WorldValue for List<T> {
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
impl<T: Widge> Prefab for List<T> {
    type Param = T::Param;

    fn spawn(&self, mut commands: EntityCommands, param: &mut SystemParamItem<Self::Param>) {
        commands.with_children(|commands| {
            for elem in &self.items {
                let commands = commands.spawn(ListItem);
                elem.spawn(commands, param);
            }
        });
    }
}
impl<T: Widge> Widge for List<T> {}
impl<T: ExtractPrefab + Widge> ExtractPrefab for List<T> {
    type ExtractParam<'w, 's> = (
        Query<'w, 's, Entity, With<ListItem>>,
        Query<'w, 's, &'static Children>,
        T::ExtractParam<'w, 's>,
    );

    fn extract(
        entity: Entity,
        (items, children, param): &SystemParamItem<Self::ExtractParam<'_, '_>>,
    ) -> Option<Self> {
        let items = items
            .iter_many(children.get(entity).ok()?)
            .map(|item| T::extract(item, param))
            .collect::<Option<_>>()?;
        Some(List { items })
    }
}
