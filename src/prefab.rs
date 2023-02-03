use bevy::{
    ecs::system::{Command, EntityCommands, SystemParam, SystemParamItem, SystemState},
    prelude::{Commands, Entity, World},
};

/// Trait that marks a struct as a Prefab
pub trait Prefab {
    type Param: SystemParam;
    fn spawn(&self, commands: EntityCommands, param: &mut SystemParamItem<Self::Param>);
}

/// Command that can be inserted to Commands
struct InsertPrefab<T: Prefab> {
    prefab: T,
    entity: Entity,
}

impl<T: Prefab + Send + Sync + 'static> Command for InsertPrefab<T> {
    fn write(self, world: &mut World) {
        let mut state: SystemState<(Commands, T::Param)> = SystemState::new(world);
        let (mut commands, param) = state.get_mut(world);
        let e_commands = commands.entity(self.entity);
        self.prefab.spawn(e_commands, &mut param);
        state.apply(world);
    }
}

//===== EXTENSIONS TO COMMANDS / ENTITY_COMMANDS =======
pub trait InsertPrefabCommand {
    fn insert_prefab<T: Prefab + Send + Sync + 'static>(&mut self, prefab: T) -> &mut Self;
}

impl<'w, 's, 'a> InsertPrefabCommand for EntityCommands<'w, 's, 'a> {
    fn insert_prefab<T: Prefab + Send + Sync + 'static>(&mut self, prefab: T) -> &mut Self {
        let entity = self.id();
        self.commands().add(InsertPrefab { prefab, entity });
        self
    }
}

pub trait SpawnPrefabCommand<'w, 's> {
    fn spawn_prefab<'a, T: Prefab + Send + Sync + 'static>(
        &'a mut self,
        prefab: T,
    ) -> EntityCommands<'w, 's, 'a>;
}
impl<'w, 's> SpawnPrefabCommand<'w, 's> for Commands<'w, 's> {
    fn spawn_prefab<'a, T: Prefab + Send + Sync + 'static>(
        &'a mut self,
        prefab: T,
    ) -> EntityCommands<'w, 's, 'a> {
        let entity = self.spawn_empty().id();
        self.add(InsertPrefab { prefab, entity });
        return self.entity(entity);
    }
}
