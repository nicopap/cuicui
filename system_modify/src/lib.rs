mod access;
mod access_registry;
mod id;
mod path;
mod split_reflect_path;

use std::marker::PhantomData;

use bevy::ecs::component::TableStorage;
use bevy::prelude::{App, Component, Mut, Resource, World};
use datazoo::RawIndexMap;
use string_interner::{backend::BucketBackend, StringInterner};

use access_registry::AccessRegistry;
pub use id::Id;

type Bitset = datazoo::Bitset<Vec<u32>>;
type Fid = Id;
type Fix = u32;

pub trait ModifierState: Component<Storage = TableStorage> {}
impl<T: Component<Storage = TableStorage>> ModifierState for T {}

pub trait ModifierFn: Send + Sync + 'static {
    fn register(&self, world: &World, builder: &mut Builder) -> (Bitset, Bitset);
    fn run(&mut self, world: &mut World);
}
trait ModifierFnMaker<T>: Send + Sync + 'static {
    fn register(&self, world: &World, builder: &mut Builder) -> (Bitset, Bitset);
    fn run(&mut self, world: &mut World);
}

struct Textend<F, T>(F, PhantomData<fn(T)>);

#[rustfmt::skip]
impl<T: 'static, F: ModifierFnMaker<T>> ModifierFn for Textend<F, T> {
    fn register(&self, world: &World, builder: &mut Builder) -> (Bitset, Bitset) { self.0.register(world, builder) }
    fn run(&mut self, world: &mut World) { self.0.run(world) }
}

#[derive(Resource)]
pub struct Builder {
    interner: StringInterner<BucketBackend<Fid>>,
    f_map: RawIndexMap<Fid, Fix>,
    fns: Vec<Modifier>,

    internal_world: World,
    reg: AccessRegistry,
}

#[derive(Resource)]
pub struct Modifiers {
    fns: Box<[Modifier]>,
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            interner: StringInterner::new(),
            f_map: RawIndexMap::default(),
            fns: Vec::new(),
            internal_world: World::new(),
            reg: AccessRegistry::default(),
        }
    }
}
impl Builder {
    pub fn add<F: ModifierFn>(&mut self, world: &World, name: &'static str, function: F) {
        let already_exists = self.interner.get(name).is_some();
        if already_exists {
            return; // TODO(err): Show to user there is a conflicting name.
        }
        let id = self.interner.get_or_intern_static(name);
        self.f_map.set_expanding(&id, &(self.fns.len() as u32));

        let (depends, changes) = function.register(world, self);

        let modifier = Modifier { depends, changes, function: Box::new(function) };
        self.fns.push(modifier)
    }
    pub fn finish(self) -> Modifiers {
        Modifiers { fns: self.fns.into_boxed_slice() }
    }
}
pub trait AppExt {
    fn add_modifer<F: ModifierFn>(&mut self, name: &'static str, f: F);
}
impl AppExt for App {
    fn add_modifer<F: ModifierFn>(&mut self, name: &'static str, f: F) {
        if !self.world.contains_resource::<Builder>() {
            self.world.init_resource::<Builder>();
        }
        self.world.resource_scope(|world, mut mods: Mut<Builder>| {
            mods.add(world, name, f);
        })
    }
}
