//! Stores [`Modify`].

use std::{collections::BTreeMap, fmt, mem};

use anyhow::anyhow;
use datazoo::SortedPairIterator;
use smallvec::SmallVec;
use string_interner::{backend::StringBackend, StringInterner, Symbol};

#[cfg(doc)]
use crate::prefab::Modify;

use crate::prefab::Prefab;

/// A binding id used in [`World`] and [`Local`] to associate a name to a
/// [`Modify`].
///
/// [`World`] [interns] strings used to identify bindings for efficiency.
///
/// [interns]: https://en.wikipedia.org/wiki/String_interning
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(crate) u32);

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<B{}>", self.0)
    }
}

#[derive(Debug)]
pub struct Local<P: Prefab> {
    bindings: BTreeMap<Id, (bool, P::Modify)>,
    buffered: Vec<(Box<str>, P::Modify)>,
    resolved: SmallVec<[(Box<str>, Id); 2]>,
}
#[derive(Debug)]
pub struct World<P: Prefab> {
    bindings: BTreeMap<Id, (bool, P::Modify)>,
    interner: StringInterner<StringBackend<Id>>,
}
impl<P: Prefab> Default for Local<P> {
    fn default() -> Self {
        Local {
            bindings: Default::default(),
            buffered: Default::default(),
            resolved: Default::default(),
        }
    }
}
impl<P: Prefab> Default for World<P> {
    fn default() -> Self {
        World {
            bindings: BTreeMap::default(),
            interner: StringInterner::new(),
        }
    }
}
#[derive(Clone, Copy)]
pub struct View<'a, P: Prefab> {
    root: &'a BTreeMap<Id, (bool, P::Modify)>,
    overlay: Option<&'a BTreeMap<Id, (bool, P::Modify)>>,
}

impl<P: Prefab> Local<P> {
    pub fn set_by_id(&mut self, id: Id, value: P::Modify) {
        self.bindings.insert(id, (true, value));
    }
    pub fn set(&mut self, binding_name: impl Into<String>, value: P::Modify) {
        let name = binding_name.into();
        let is_name = |(known, _): &(Box<str>, _)| known.as_ref().cmp(&name);
        if let Ok(resolved) = self.resolved.binary_search_by(is_name) {
            let id = self.resolved[resolved].1;
            self.bindings.insert(id, (true, value));
        } else {
            self.buffered.push((name.into(), value));
        }
    }
    pub fn sync(&mut self, global: &World<P>) -> anyhow::Result<()> {
        let Local { buffered, bindings: inner, resolved } = self;
        for (new_name, new_modify) in buffered.drain(..) {
            let err = || anyhow!("tried to bind a name that doesn't exist");
            let id = global.interner.get(&*new_name).ok_or_else(err)?;
            if let Err(index) = resolved.binary_search_by_key(&&new_name, |t| &t.0) {
                resolved.insert(index, (new_name, id));
            }
            inner.insert(id, (true, new_modify));
        }
        Ok(())
    }

    pub fn reset_changes(&mut self) {
        self.bindings.values_mut().for_each(|v| v.0 = false);
    }
}
impl<P: Prefab> World<P> {
    // TODO(err): Should return Result
    pub fn set(&mut self, key: &str, value: P::Modify) -> Option<()> {
        let id = self.interner.get(key)?;
        self.set_id(id, value);
        Some(())
    }
    pub fn set_id(&mut self, id: Id, value: P::Modify) {
        self.bindings.insert(id, (true, value));
    }
    /// Like `set` but do not mark as modified if `value` is same as previous
    /// value for `key`.
    pub fn set_neq(&mut self, key: &str, value: P::Modify) -> Option<()>
    where
        P::Modify: PartialEq,
    {
        let id = self.interner.get(key)?;
        self.set_id_neq(id, value);
        Some(())
    }
    /// Like `set_id` but do not mark as modified if `value` is same as previous
    /// value for `id`.
    pub fn set_id_neq(&mut self, id: Id, value: P::Modify)
    where
        P::Modify: PartialEq,
    {
        match self.bindings.get_mut(&id) {
            Some(old_value) => {
                if old_value.1 != value {
                    *old_value = (true, value);
                }
            }
            None => {
                self.bindings.insert(id, (true, value));
            }
        }
    }
    pub fn get_or_add(&mut self, name: impl AsRef<str>) -> Id {
        self.interner.get_or_intern(name)
    }
    pub fn view(&self) -> View<P> {
        View { root: &self.bindings, overlay: None }
    }
    pub fn view_with_local<'a>(&'a self, local: &'a mut Local<P>) -> anyhow::Result<View<'a, P>> {
        local.sync(self)?;
        Ok(View {
            overlay: Some(&local.bindings),
            root: &self.bindings,
        })
    }

    pub fn reset_changes(&mut self) {
        self.bindings.values_mut().for_each(|v| v.0 = false);
    }
}
impl<'a, P: Prefab> View<'a, P> {
    pub(crate) fn changed(&self) -> impl Iterator<Item = (&Id, &P::Modify)> + '_ {
        // Due to Rust's poor type inference on closures, I must write this inline:
        // let changed = |(changed, modify): &(bool, _)| changed.then_some(modify);
        let overlay = self.overlay.iter().flat_map(|b| *b);
        let overlay = overlay.filter_map_values(|(c, m)| c.then(|| m));
        let root = self.root.iter().filter_map_values(|(c, m)| c.then(|| m));

        overlay.outer_join(root).filter_map_values(|(l, r)| l.or(r))
    }
    pub fn get(&self, id: Id) -> Option<&'a P::Modify> {
        self.overlay
            .and_then(|btm| btm.get(&id))
            .or_else(|| self.root.get(&id))
            .map(|(_c, modify)| modify)
    }
}

impl Symbol for Id {
    fn try_from_usize(index: usize) -> Option<Self> {
        let u32 = u32::try_from(index).ok()?;
        Some(Id(u32))
    }

    fn to_usize(self) -> usize {
        assert!(
            mem::size_of::<usize>() >= mem::size_of::<Self>(),
            "NOTE: please open an issue if you need to run bevy on 16 bits plateforms"
        );
        self.0 as usize
    }
}
