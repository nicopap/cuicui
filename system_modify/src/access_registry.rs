use std::mem;

use bevy::{
    ecs::component::ComponentId,
    prelude::{Component, World},
    utils::HashMap,
};
use datazoo::Bitset;

pub(crate) type Aix = u32;

#[derive(Debug, Default)]
pub struct AccessRegistry {
    depends: Vec<Bitset<Box<[u32]>>>,
    changes: Vec<Bitset<Box<[u32]>>>,

    // TODO(bug): Hande atomic access
    a_map: HashMap<(ComponentId, &'static str), Aix>,
}
impl AccessRegistry {
    pub fn record<'a, 'b>(&'a mut self, world: &'b mut World) -> FnAccessRecorder<'a, 'b> {
        FnAccessRecorder {
            reg: self,
            depends: Bitset::default(),
            changes: Bitset::default(),
            world,
        }
    }
}
pub struct FnAccessRecorder<'a, 'b> {
    reg: &'a mut AccessRegistry,
    world: &'b mut World,
    depends: Bitset<Vec<u32>>,
    changes: Bitset<Vec<u32>>,
}
pub struct CompAccessRecorder<'a, 'b> {
    reg: &'a mut AccessRegistry,
    world: &'b mut World,
    depends: &'a mut Bitset<Vec<u32>>,
    changes: &'a mut Bitset<Vec<u32>>,
    cid: ComponentId,
}
impl<'a, 'b> CompAccessRecorder<'a, 'b> {
    fn aix(&mut self, path: &'static str) -> usize {
        let aix = self.reg.a_map.len() as Aix;
        *self.reg.a_map.entry((self.cid, path)).or_insert(aix) as usize
    }
    pub fn read(&mut self, path: &'static str) {
        self.depends.enable_bit_extending(self.aix(path));
    }
    pub fn write(&mut self, path: &'static str) {
        self.changes.enable_bit_extending(self.aix(path));
    }
}
impl<'a, 'b> FnAccessRecorder<'a, 'b> {
    pub fn for_component<'c, C: Component>(&'c mut self) -> CompAccessRecorder<'c, 'b> {
        let world = self.world;
        let cid = world
            .component_id::<C>()
            .unwrap_or_else(|| world.init_component::<C>());

        CompAccessRecorder {
            reg: self.reg,
            world,
            depends: &mut self.depends,
            changes: &mut self.changes,
            cid,
        }
    }
}
impl<'a, 'b> Drop for FnAccessRecorder<'a, 'b> {
    fn drop(&mut self) {
        let Self { reg, depends, changes, .. } = self;
        reg.depends
            .push(Bitset(mem::take(depends).0.into_boxed_slice()));
        reg.changes
            .push(Bitset(mem::take(changes).0.into_boxed_slice()));
    }
}
