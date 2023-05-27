use std::collections::BTreeMap;

use super::Id;

pub enum Entry<'a, V> {
    Occupied(&'a mut V),
    Vacant(Id, &'a mut BTreeMap<Id, (bool, V)>),
}
impl<'a, V> Entry<'a, V> {
    pub(super) fn new(parent: &'a mut BTreeMap<Id, (bool, V)>, key: Id) -> Self {
        if parent.contains_key(&key) {
            let (change, value) = parent.get_mut(&key).unwrap();
            *change = true;
            Entry::Occupied(value)
        } else {
            Entry::Vacant(key, parent)
        }
    }
    pub fn modify(mut self, f: impl FnOnce(&mut V)) -> Self {
        if let Entry::Occupied(entry) = &mut self {
            f(*entry);
        }
        self
    }
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }
    pub fn or_insert(self, default: V) -> &'a mut V {
        self.or_insert_with(|| default)
    }
    pub fn or_insert_with(self, default: impl FnOnce() -> V) -> &'a mut V {
        match self {
            Entry::Occupied(value) => value,
            Entry::Vacant(key, parent) => {
                parent.insert(key, (true, default()));
                &mut parent.get_mut(&key).unwrap().1
            }
        }
    }
}
