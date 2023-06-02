use datazoo::sorted;

use super::Id;

pub enum Entry<'a, V> {
    Occupied(&'a mut V),
    Vacant(Id, &'a mut sorted::ByKeyVec<Id, (bool, V)>),
}
impl<'a, V> Entry<'a, V> {
    pub(super) fn new(parent: &'a mut sorted::ByKeyVec<Id, (bool, V)>, key: Id) -> Self {
        if parent.contains_key(&key) {
            let (change, value) = parent.get_mut(&key).unwrap();
            *change = true;
            Entry::Occupied(value)
        } else {
            Entry::Vacant(key, parent)
        }
    }
    /// Update the `Entry` if it's already occupied, does nothing if it is vacant.
    pub fn modify(mut self, f: impl FnOnce(&mut V)) -> Self {
        if let Entry::Occupied(entry) = &mut self {
            f(*entry);
        }
        self
    }
    /// If this `Entry` is vacant, set it to its default value, returns entry.
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }
    /// If this `Entry` is vacant, set it to `default`, returns entry.
    pub fn or_insert(self, default: V) -> &'a mut V {
        self.or_insert_with(|| default)
    }
    /// If this `Entry` is vacant, set it to `default`, returns entry.
    ///
    /// `default` is only evaluated if the entry is vacant.
    pub fn or_insert_with(self, default: impl FnOnce() -> V) -> &'a mut V {
        match self {
            Entry::Occupied(value) => value,
            Entry::Vacant(key, parent) => {
                parent.insert(key, (true, default()));
                &mut parent.get_mut(&key).unwrap().1
            }
        }
    }
    /// Set this entry to `new_value`, even if already occupied.
    pub fn insert(self, new_value: V) -> &'a mut V {
        match self {
            Entry::Occupied(value) => {
                *value = new_value;
                value
            }
            Entry::Vacant(key, parent) => {
                parent.insert(key, (true, new_value));
                &mut parent.get_mut(&key).unwrap().1
            }
        }
    }
}
