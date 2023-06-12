//! Define wrapper types & traits for bevy's `QueryState` and `QueryIter`.
//!
//! The concrete type is erased by defining traits for the `QueryState` and `QueryIter`
//! and wrapping them into a trait object (`Box<dyn _>`).
//!
//! There is an individual wrapper type per [`crate::ReflectQueryable`] `query` method.
use std::iter;

use bevy::ecs::query::{QueryIter, QuerySingleError};
use bevy::prelude::{Component, Entity, Mut, QueryState, Ref as BRef, Reflect, With, World};

type SingleResult<T> = Result<T, QuerySingleError>;

use crate::custom_ref::Ref;
#[cfg(doc)]
use crate::ReflectQueryable;

macro_rules! impl_iter {
    (
        // module to define trait and structs internally used by the iter
        mod $smod:ident;

        // Item description and documentation
        item: [$item_doc:literal, $item_doc_link:literal, $item_ty:ty];

        // The iterable type, one returned by `iter` method on the state type
        iter: pub struct $iter:ident(Box<dyn _>);

        // The state type, the one returned by the ReflectQueryable methods
        state: pub struct $state:ident(Box<dyn _>);

        // type of `world` param of $state `iter` method.
        world_ty: $world_ty:ty;

        // Body of the `iter` method on $state
        iter_method: $iter_method:ident;
        get_method: $get_method:ident;
        $(iter_map: $iter_map:expr;)?

        impl QueryState [$($query_state_param_ty:tt)*];

        impl [$($concrete_iter_params:tt)*] Iter for $concrete_iter_ty:ty
    ) => {
        #[doc = concat!(
"An iterator over all [`", $item_doc, "`]($", $item_doc_link, ") in a `world`.

This iterates over entities in the `world` provided
to [`", stringify!($state), "`], for a [`ReflectQueryable`] component."
        )]
        pub struct $iter<'a, 'w: 'a, 's: 'a>(Box<dyn $smod::Iter<'w, 's> + 'a>);

        impl<'a, 'w: 'a, 's: 'a> ExactSizeIterator for $iter<'a, 'w, 's> {
            fn len(&self) -> usize { self.0.len() }
        }
        impl<'a, 'w: 'a, 's: 'a> Iterator for $iter<'a, 'w, 's> {
            type Item = $item_ty;

            fn next(&mut self) -> Option<Self::Item> { self.0.next() }
            fn size_hint(&self) -> (usize, Option<usize>) { self.0.size_hint() }
        }

        #[doc = concat!(
"An erased [`QueryState`] to iterate over [`", $item_doc, "`](", $item_doc_link, ").

Use [`", stringify!($state), ".iter(world)`](Self::iter) to iterate over all
[`", $item_doc, "`](", $item_doc_link, ") with the underlying [`Component`]
of the [`ReflectQueryable`](crate::ReflectQueryable) this [`", stringify!($state), "`] was
created from."
        )]
        pub struct $state (pub(super) Box<dyn $smod::State>);

        #[allow(clippy::redundant_closure_call)]
        #[allow(clippy::redundant_closure_for_method_calls)]
        impl<C: Component + Reflect> $smod::State for QueryState <$($query_state_param_ty)*> {
            fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: $world_ty) -> $iter<'a, 'w, 's> {
                $iter(Box::new(
                    self .$iter_method(world) $(.map($iter_map))?
                ))
            }

            fn get_single<'w, 's>(&'s mut self, world: $world_ty) -> SingleResult<$item_ty> {
                self.$get_method(world)$(.map($iter_map))?
            }
        }

        impl $state {
            #[doc = concat!("Get an iterator over [`", $item_doc, "`](", $item_doc_link, ").")]
            pub fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: $world_ty) -> $iter<'a, 'w, 's> {
                self.0.iter(world)
            }
            #[doc = concat!("Get a single [`", $item_doc, "`](", $item_doc_link, ").\n\
            \n\
            Return an `Err` if there isn't exactly one such thing in `world`.")]
            pub fn get_single<'w, 's>(&'s mut self, world: $world_ty) -> SingleResult<$item_ty> {
                self.0.get_single(world)
            }
        }
        mod $smod {
            use super::*;

            // Traits for wrapped erased values.
            pub trait State {
                fn iter<'a, 'w: 'a, 's: 'a>(&'s mut self, world: $world_ty) -> $iter<'a, 'w, 's>;
                fn get_single<'w, 's>(&'s mut self, world: $world_ty) -> SingleResult<$item_ty>;
            }
            pub trait Iter<'w, 's>: ExactSizeIterator<Item = $item_ty> {}

            impl<'w, 's, $($concrete_iter_params)*> Iter<'w, 's> for $concrete_iter_ty {}
        }
    };
}

impl_iter! {
    mod wrapper;
    item: ["&dyn Reflect", "Reflect", &'w dyn Reflect];
    iter:  pub struct QuerydynIter(Box<dyn _>);
    state: pub struct Querydyn(Box<dyn _>);
    world_ty: &'w World;
    iter_method: iter;
    get_method: get_single;
    iter_map: C::as_reflect;

    impl QueryState[&'static C, ()];

    impl[ C: Component + Reflect, F: Fn(&C) -> &dyn Reflect ] Iter
    for iter::Map<QueryIter<'w, 's, &'static C, ()>, F>
}

fn map_ref<C: Component + Reflect>(value: BRef<C>) -> Ref<dyn Reflect> {
    Ref::map_from(value, C::as_reflect)
}
impl_iter! {
    mod ref_wrapper;
    item: ["Ref<dyn Reflect>", "Ref", Ref<'w, dyn Reflect>];
    iter:  pub struct RefQuerydynIter(Box<dyn _>);
    state: pub struct RefQuerydyn(Box<dyn _>);
    world_ty:&'w World;
    iter_method: iter;
    get_method: get_single;
    iter_map: map_ref;

    impl QueryState[BRef<'static, C>, ()] ;

    impl[ C: Component + Reflect, F: Fn(BRef<C>) -> Ref<dyn Reflect> ] Iter
    for iter::Map<QueryIter<'w, 's, BRef<'static , C>, ()>, F>
}

impl_iter! {
    mod entity_wrapper;
    item: ["Entity", "Entity", Entity];
    iter:  pub struct EntityQuerydynIter(Box<dyn _>);
    state: pub struct EntityQuerydyn(Box<dyn _>);
    world_ty:&'w World;
    iter_method: iter;
    get_method: get_single;

    impl QueryState[Entity, With<C>] ;

    impl[ C: Component] Iter for QueryIter<'w, 's, Entity, With<C>>
}

fn map_unchanged<C: Component + Reflect>(value: Mut<C>) -> Mut<dyn Reflect> {
    value.map_unchanged(C::as_reflect_mut)
}
impl_iter! {
    mod mut_wrapper;
    item: ["Mut<dyn Reflect>", "Mut", Mut<'w, dyn Reflect>];
    iter:  pub struct MutQuerydynIter(Box<dyn _>);
    state: pub struct MutQuerydyn(Box<dyn _>);
    world_ty:&'w mut World;
    iter_method: iter_mut;
    get_method: get_single_mut;
    iter_map: map_unchanged;

    impl QueryState[&'static mut C, ()] ;


    impl[C: Component + Reflect, F: Fn(Mut<C>) -> Mut<dyn Reflect>] Iter
    for iter::Map<QueryIter<'w, 's, &'static mut C, ()>, F>
}
