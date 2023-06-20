mod access;
mod access_registry;
mod id;
mod path;
mod split_reflect_path;

use bevy::prelude::{App, Mut, Resource, World};
use datazoo::RawIndexMap;
use string_interner::{backend::BucketBackend, StringInterner};

pub use access::Set;
use access_registry::{AccessRegistry, FnAccessRecorder};
pub use id::Id;
pub use path::{IntoModifierState, ModifierBox};

type Fid = Id;
type Fix = u32;

pub struct ModifierInit<F, I> {
    pub function: F,
    pub init_data: I,
}

#[derive(Resource)]
pub struct Builder {
    interner: StringInterner<BucketBackend<Fid>>,
    f_map: RawIndexMap<Fid, Fix>,
    fns: Vec<ModifierBox>,

    internal_world: World,
    reg: AccessRegistry,
}

#[derive(Resource)]
pub struct Modifiers {
    fns: Box<[ModifierBox]>,
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
    fn record<T>(
        &mut self,
        world: &mut World,
        f: impl FnOnce(&mut World, FnAccessRecorder) -> T,
    ) -> T {
        let rec = self.reg.record(world);
        let internal_world = &mut self.internal_world;

        f(internal_world, rec)
    }
    pub fn add<T, M, S>(&mut self, world: &mut World, name: &'static str, function: M, init: S)
    where
        M: IntoModifierState<T, InitData = S>,
    {
        let already_exists = self.interner.get(name).is_some();
        if already_exists {
            return; // TODO(err): Show to user there is a conflicting name.
        }
        let id = self.interner.get_or_intern_static(name);
        let f_id = self.fns.len() as u32;
        self.f_map.set_expanding(&id, &f_id);
        let modifier = self.record(world, |internal_world, mut rec| {
            function.into_modifier_state(internal_world, &mut rec, init)
        });
        self.fns.push(modifier)
    }
    pub fn finish(self) -> Modifiers {
        Modifiers { fns: self.fns.into_boxed_slice() }
    }
}
pub trait AppExt {
    fn add_modifer<T, M, S>(&mut self, name: &'static str, f: M, init: S)
    where
        M: IntoModifierState<T, InitData = S>;
}
impl AppExt for App {
    fn add_modifer<T, M, S>(&mut self, name: &'static str, f: M, init: S)
    where
        M: IntoModifierState<T, InitData = S>,
    {
        if !self.world.contains_resource::<Builder>() {
            self.world.init_resource::<Builder>();
        }
        self.world.resource_scope(|world, mut mods: Mut<Builder>| {
            mods.add(world, name, f, init);
        })
    }
}
