use std::marker::PhantomData;

use bevy::prelude::{Component, Entity, Mut, Reflect, World};

use crate::access::{item_from_query, Access, Item};
use crate::access_registry::FnAccessRecorder;

pub type ModifierBox = Box<dyn Modifier + Send + Sync + 'static>;

pub struct State<'a, C>(pub Mut<'a, C>);

#[derive(Component)]
struct StateStore<T>(T);

impl<'a, C: Send + Sync + 'static> State<'a, C> {
    fn init(internal_world: &mut World, data: C) -> Entity {
        internal_world.spawn(StateStore(data)).id()
    }
    fn fetch(state: Entity, internal_world: &'a mut World) -> Self {
        let w = internal_world;
        let data = w.query::<&mut StateStore<C>>().get_mut(w, state).unwrap();
        State(data.map_unchanged(|d| &mut d.0))
    }
}
impl<C: Component + Reflect, A: Access> Item<C, A> {
    fn init<'a: 'c, 'b: 'c, 'c>(
        stuff: &'c mut FnAccessRecorder<'a, 'b>,
        data: A::Paths,
    ) -> A::ParsedPaths {
        Self::record(&data, stuff);
        A::parse_path(data)
    }
}
fn item_fetch<'w, C: Component + Reflect, A: Access>(
    state: &mut A::ParsedPaths,
    item: Mut<'w, C>,
) -> Item<C, A::Concrete<'w>> {
    item_from_query::<C, A>(item, state)
}
// impl<C: Component + Reflect, A: Access> Item<C, A> {
//     fn init<'z>(stuff: &mut FnAccessRecorder, data: A::Paths) -> A::ParsedPaths {
//         Self::record(&data, stuff);
//         A::parse_path(data)
//     }
//     fn fetch<'z>(state: &mut A::ParsedPaths, entity: Entity, world: &'z mut World) -> Self
//     where
//         A: Access<Concrete<'z> = A>,
//     {
//         let item = world.query::<&mut C>().get_mut(world, entity).unwrap();
//         item_from_query::<C, A>(item, state)
//     }
// }

pub trait Modifier {
    fn state(&self) -> Option<Entity>;
    // TODO(clean) instead of `internal_world` consider newtyping World
    fn run(&mut self, entity: Entity, world: &mut World, internal_world: &mut World);
}

struct ModifierState<F, S, Dummy> {
    function: F,
    state: S,
    _dummy: PhantomData<fn(Dummy)>,
}

pub trait IntoModifierState<T> {
    type InitData;

    fn into_modifier_state<'a: 'c, 'b: 'c, 'c>(
        self,
        internal_world: &mut World,
        rec: &'c mut FnAccessRecorder<'a, 'b>,
        data: Self::InitData,
    ) -> ModifierBox;
}
pub fn can_be_modifier_state<M, S, T>()
where
    M: IntoModifierState<T, InitData = S>,
{
}
pub fn can_be_modifier_item<C, A>()
where
    C: Component + Reflect,
    A: Access + 'static,
{
}

#[rustfmt::skip]
mod impls {
    use super::*;
    
    // this allows creating an expression with same arity as $t but that doesn't contain $t
    macro_rules! for_t { ($t:tt, $($what:tt)*) => { $($what)* }; }

    macro_rules! make_modifier_state {
        (
            $( $s:ident,)?
            $([ $c_i:ident, $a_i:ident, $i:tt , $ii:tt]),*
        ) => {
            impl<F, $($s,)? $($c_i, $a_i,)*>  IntoModifierState<( $($s,)? $($c_i, $a_i,)* )> for F
            where
                F: FnMut( $(State<$s>,)? $(Item<$c_i, $a_i>,)* )
                 + FnMut( $(State<$s>,)? $(Item<$c_i, $a_i::Concrete<'_>>,)* ),
                F: Send + Sync + 'static,
                $($s : Send + Sync + 'static,)?
                $(
                    $c_i : Component + Reflect,
                    $a_i : Access + 'static,
                )*
            {
                type InitData = ($($s,)? $(<$a_i as Access>::Paths,)*);

                fn into_modifier_state<'a: 'c, 'b: 'c, 'c>(
                    self,
                    #[allow(unused_variables)] internal_world: &mut World,
                    rec: &'c mut FnAccessRecorder<'a, 'b>,
                    d: Self::InitData,
                ) -> ModifierBox {
                    Box::new(ModifierState {
                        function: self,
                        state: (
                            $( State::<$s>::init(internal_world, d.0), )?
                            $( Item::<$c_i, $a_i>::init(rec, d. $i) ,)*
                        ),
                        _dummy: PhantomData::<fn(($($s,)? $($c_i, $a_i,)*))>,
                    })
                }
            }
            impl<F, $($s,)? $($c_i, $a_i,)*>  Modifier for ModifierState<
                F,
                ( $(for_t![$s, Entity],)?  $(<$a_i as Access>::ParsedPaths,)* ),
                ( $($s,)? $($c_i, $a_i,)* ),
            >
            where
                F: FnMut( $(State<$s>,)? $(Item<$c_i, $a_i::Concrete<'_>>,)* ),
                $($s : Send + Sync + 'static,)?
                $(
                    $c_i : Component + Reflect,
                    $a_i : Access,
                )*
            {
                fn state(&self) -> Option<Entity> {
                    None $( .or( for_t![$s, Some(self.state.0)] ) )?
                }
                fn run(
                    &mut self,
                    entity: Entity,
                    world: &mut World,
                    #[allow(unused_variables)] internal_world: &mut World,
                ) {
                    let items = world.query::<($(&mut $c_i,)*)>().get_mut(world, entity).unwrap();
                    (self.function)(
                        $( State::<$s>::fetch(self.state.0, internal_world), )?
                        $( item_fetch::<$c_i, $a_i>(&mut self.state. $i, items. $ii), )*
                    )
                }
            }
        }
    }
    make_modifier_state!(S, [C0, A0, 1, 0]);
    make_modifier_state!(S, [C0, A0, 1, 0], [C1, A1, 2, 1]);
    make_modifier_state!(S, [C0, A0, 1, 0], [C1, A1, 2, 1], [C2, A2, 3, 2]);
    make_modifier_state!(S, [C0, A0, 1, 0], [C1, A1, 2, 1], [C2, A2, 3, 2], [C3, A3, 4, 3]);
    make_modifier_state!(S, [C0, A0, 1, 0], [C1, A1, 2, 1], [C2, A2, 3, 2], [C3, A3, 4, 3], [C4, A4, 5, 4]);
    make_modifier_state!(S, [C0, A0, 1, 0], [C1, A1, 2, 1], [C2, A2, 3, 2], [C3, A3, 4, 3], [C4, A4, 5, 4], [C5, A5, 6, 5]);
    make_modifier_state!([C0, A0, 0, 0]);
    make_modifier_state!([C0, A0, 0,0], [C1, A1, 1,1]);
    make_modifier_state!([C0, A0, 0,0], [C1, A1, 1,1], [C2, A2, 2,2]);
    make_modifier_state!([C0, A0, 0,0], [C1, A1, 1,1], [C2, A2, 2,2], [C3, A3, 3,3]);
    make_modifier_state!([C0, A0, 0,0], [C1, A1, 1,1], [C2, A2, 2,2], [C3, A3, 3,3], [C4, A4, 4,4]);
    make_modifier_state!([C0, A0, 0,0], [C1, A1, 1,1], [C2, A2, 2,2], [C3, A3, 3,3], [C4, A4, 4,4], [C5, A5, 5,5]);
}

#[cfg(test)]
mod tests {
    use crate::{access::Set, Builder};

    use super::*;
    use bevy::prelude::{Quat, Style, Transform, Val, Vec3};

    const M1: (i32, [&str; 3], &str) = (
        32,
        [".translation.x", ".scale", ".rotation"],
        ".margin.left",
    );
    fn m1(
        State(state): State<i32>,
        Item((x, scale, rot), ..): Item<Transform, (Set<f32>, &mut Vec3, &Quat)>,
        Item(left, ..): Item<Style, &mut Val>,
    ) {
        todo!();
    }

    #[test]
    fn function_parses() {
        let mut world = World::new();

        let mut builder = Builder::default();
        builder.add(&mut world, "m1", M1, m1);
        let mods = builder.finish();
    }
}
