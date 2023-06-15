use std::iter;

use enumset::{EnumSet, EnumSetType};

use crate::{
    binding::View,
    modify::{Changing, Indexed, Modify},
    resolve::{MakeModify, Resolver},
};

/// A `Modify` That does literally nothing.
///
/// This can be useful for tests or example code.
#[derive(Debug, Clone)]
pub struct DummyModify;

#[derive(EnumSetType, Debug)]
pub enum NoFields {}

impl Indexed<DummyModify> for () {
    fn get_mut(&mut self, _: usize) -> Option<&mut ()> {
        None
    }
}
impl Resolver<DummyModify> for () {
    fn new<T, F>(mods: Vec<MakeModify<DummyModify>>, f: F, _: &()) -> ((), Vec<T>)
    where
        T: for<'a> AsMut<()>,
        F: Fn() -> T,
    {
        let Some(section_count) = mods.iter().map(|m| m.range.end).max() else {
            return ((), Vec::new())
        };
        let dummies = iter::repeat_with(f).take(section_count as usize).collect();
        ((), dummies)
    }

    fn update<'a, T>(
        &'a self,
        _: &mut (),
        _: &'a Changing<NoFields, T>,
        _: View<'a, DummyModify>,
        _: &(),
    ) where
        for<'b> &'b T: Into<()>,
    {
    }
}
impl Modify for DummyModify {
    type Item<'a> = ();
    type Items<'a> = ();
    type Field = NoFields;
    type Context<'a> = ();
    type Resolver = ();

    fn apply(&self, (): &(), (): &mut ()) -> anyhow::Result<()> {
        Ok(())
    }
    fn depends(&self) -> EnumSet<NoFields> {
        EnumSet::EMPTY
    }
    fn changes(&self) -> EnumSet<Self::Field> {
        EnumSet::EMPTY
    }
}
