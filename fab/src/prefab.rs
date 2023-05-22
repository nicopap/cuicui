use std::fmt;

use enumset::{EnumSet, EnumSetType};

pub trait Indexed<T: ?Sized> {
    fn get_mut(&mut self, index: usize) -> Option<&mut T>;
}
impl<T> Indexed<T> for [T] {
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        <[T]>::get_mut(self, index)
    }
}
impl<T> Indexed<T> for Vec<T> {
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        <[T]>::get_mut(self, index)
    }
}

type Keys<M, I> = EnumSet<Key<M, I>>;
type Key<M, I> = <M as Modify<I>>::Field;
type Ctx<'a, M, I> = <M as Modify<I>>::Context<'a>;
pub type FieldsOf<P> = Keys<<P as Prefab>::Modifiers, <P as Prefab>::Item>;
pub type Field<P> = Key<<P as Prefab>::Modifiers, <P as Prefab>::Item>;
pub type Context<'a, P> = Ctx<'a, <P as Prefab>::Modifiers, <P as Prefab>::Item>;

pub trait Modify<I: ?Sized> {
    type Field: EnumSetType;
    type Context<'a>
    where
        Self: 'a;

    /// Apply this modifier to the `item`.
    fn apply(&self, ctx: &Self::Context<'_>, section: &mut I) -> anyhow::Result<()>;

    /// On what data in `item` does this modifier depends?
    fn depends(&self) -> EnumSet<Self::Field>;

    /// What data does this `Modify` changes?
    fn changes(&self) -> EnumSet<Self::Field>;
}

pub trait Prefab {
    type Modifiers: Modify<Self::Item> + fmt::Debug;
    type Item;
    type Items: Indexed<Self::Item>;
}
pub struct Tracked<P: Prefab> {
    pub(crate) updated: FieldsOf<P>,
    pub(crate) value: P::Item,
}
impl<P: Prefab> Tracked<P> {
    pub fn new(value: P::Item) -> Self {
        Self { updated: EnumSet::EMPTY, value }
    }
    /// Update `self` with `f`, declaring that `update` is changed.
    ///
    /// If you change fields other than the ones in `updated`, they won't be
    /// tracked as changed. So make sure to properly declare which fields
    /// you are changing.
    pub fn update(&mut self, updated: FieldsOf<P>, f: impl FnOnce(&mut Self)) {
        self.updated |= updated;
        f(self);
    }
    /// Reset the change tracker state.
    pub fn reset_updated(&mut self) {
        self.updated = EnumSet::EMPTY;
    }
}
