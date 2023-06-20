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
pub struct FnAccessRecorder<'a, 'w> {
    reg: &'a mut AccessRegistry,
    world: &'w mut World,
    depends: Bitset<Vec<u32>>,
    changes: Bitset<Vec<u32>>,
}
pub struct CompAccessRecorder<'a, 'f, 'w> {
    fn_rec: &'f mut FnAccessRecorder<'a, 'w>,
    cid: ComponentId,
}
impl<'a, 'b, 'w> CompAccessRecorder<'a, 'b, 'w> {
    fn aix(&mut self, path: &'static str) -> usize {
        let aix = self.fn_rec.reg.a_map.len() as Aix;
        *self.fn_rec.reg.a_map.entry((self.cid, path)).or_insert(aix) as usize
    }
    pub fn read(&mut self, path: &'static str) {
        let aix = self.aix(path);
        self.fn_rec.depends.enable_bit_extending(aix);
    }
    pub fn write(&mut self, path: &'static str) {
        let aix = self.aix(path);
        self.fn_rec.changes.enable_bit_extending(aix);
    }
}
impl<'a, 'w> FnAccessRecorder<'a, 'w> {
    pub fn for_component<'f, C: Component>(&'f mut self) -> CompAccessRecorder<'a, 'f, 'w> {
        let cid = self
            .world
            .component_id::<C>()
            .unwrap_or_else(|| self.world.init_component::<C>());

        CompAccessRecorder { fn_rec: self, cid }
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
