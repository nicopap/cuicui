//! Stores [`Modify`].

mod entry;

use std::{fmt, mem, num::NonZeroU32};

use anyhow::anyhow;
use datazoo::{index_multimap::Index, sorted, SortedPairIterator};
use smallvec::SmallVec;
use string_interner::{backend::StringBackend, StringInterner, Symbol};

#[cfg(doc)]
use crate::Modify;

pub use entry::Entry;

/// A binding id used in [`World`] and [`Local`] to associate a name to a
/// [`Modify`].
///
/// [`World`] [interns] strings used to identify bindings for efficiency.
///
/// [interns]: https://en.wikipedia.org/wiki/String_interning
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Hash, Ord)]
pub struct Id(pub(crate) NonZeroU32);
impl Index for Id {
    #[inline]
    fn get(&self) -> usize {
        self.0.get() as usize - 1
    }
}
impl Symbol for Id {
    #[inline]
    fn try_from_usize(index: usize) -> Option<Self> {
        let u32 = u32::try_from(index).ok()?;
        Some(Id(NonZeroU32::new(u32.saturating_add(1)).unwrap()))
    }

    #[inline]
    fn to_usize(self) -> usize {
        assert!(
            mem::size_of::<usize>() >= mem::size_of::<Self>(),
            "NOTE: please open an issue if you need to run bevy on 16 bits plateforms"
        );
        Index::get(&self)
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<B{}>", self.0)
    }
}

#[derive(Debug)]
pub struct Local<M> {
    bindings: sorted::ByKeyVec<Id, (bool, M)>,
    buffered: Vec<(Box<str>, M)>,
    resolved: SmallVec<[(Box<str>, Id); 2]>,
}
#[derive(Debug)]
pub struct World<M> {
    bindings: sorted::ByKeyVec<Id, (bool, M)>,
    interner: StringInterner<StringBackend<Id>>,
}
impl<M> Default for Local<M> {
    fn default() -> Self {
        Local {
            bindings: Default::default(),
            buffered: Default::default(),
            resolved: Default::default(),
        }
    }
}
impl<M> Default for World<M> {
    fn default() -> Self {
        World {
            bindings: Default::default(),
            interner: StringInterner::new(),
        }
    }
}
pub struct View<'a, M> {
    root: &'a sorted::ByKeyVec<Id, (bool, M)>,
    overlay: Option<&'a sorted::ByKeyVec<Id, (bool, M)>>,
}
impl<'a, M> Clone for View<'a, M> {
    fn clone(&self) -> Self {
        View { root: self.root, overlay: self.overlay }
    }
}
impl<'a, M> Copy for View<'a, M> {}

impl<M> Local<M> {
    pub fn entry(&mut self, id: Id) -> Entry<M> {
        Entry::new(&mut self.bindings, id)
    }
    pub fn set_by_id(&mut self, id: Id, value: M) {
        self.bindings.insert(id, (true, value));
    }
    pub fn get_mut(&mut self, binding_name: impl Into<String>) -> Option<&mut M> {
        let name = binding_name.into();
        let is_name = |(known, _): &(Box<str>, _)| known.as_ref().cmp(&name);
        let resolved = self.resolved.binary_search_by(is_name).ok()?;
        let id = self.resolved[resolved].1;
        let (changed, modify) = self.bindings.get_mut(&id)?;
        *changed = true;
        Some(modify)
    }
    pub fn set(&mut self, binding_name: impl Into<String>, value: M) {
        let name = binding_name.into();
        let is_name = |(known, _): &(Box<str>, _)| known.as_ref().cmp(&name);
        if let Ok(resolved) = self.resolved.binary_search_by(is_name) {
            let id = self.resolved[resolved].1;
            self.bindings.insert(id, (true, value));
        } else {
            self.buffered.push((name.into(), value));
        }
    }
    pub fn sync(&mut self, global: &World<M>) -> anyhow::Result<()> {
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
impl<M> World<M> {
    pub fn entry(&mut self, id: Id) -> Entry<M> {
        Entry::new(&mut self.bindings, id)
    }

    // TODO(err): Should return Result
    pub fn set(&mut self, key: &str, value: M) -> Option<()> {
        let id = self.interner.get(key)?;
        self.set_id(id, value);
        Some(())
    }
    pub fn set_id(&mut self, id: Id, value: M) {
        self.bindings.insert(id, (true, value));
    }
    /// Like `set` but do not mark as modified if `value` is same as previous
    /// value for `key`.
    pub fn set_neq(&mut self, key: &str, value: M) -> Option<()>
    where
        M: PartialEq,
    {
        let id = self.interner.get(key)?;
        self.set_id_neq(id, value);
        Some(())
    }
    /// Like `set_id` but do not mark as modified if `value` is same as previous
    /// value for `id`.
    pub fn set_id_neq(&mut self, id: Id, value: M)
    where
        M: PartialEq,
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
    /// Access mutably an existing binding. This sets the `change` bit unconditionally.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut M> {
        let id = self.interner.get(key)?;
        let (changed, modify) = self.bindings.get_mut(&id)?;
        *changed = true;
        Some(modify)
    }
    pub fn get_or_add(&mut self, name: impl AsRef<str>) -> Id {
        self.interner.get_or_intern(name)
    }
    pub fn get_id(&self, name: impl AsRef<str>) -> Option<Id> {
        self.interner.get(name)
    }
    pub fn view(&self) -> View<M> {
        View { root: &self.bindings, overlay: None }
    }
    pub fn view_with_local<'a>(&'a self, local: &'a mut Local<M>) -> anyhow::Result<View<'a, M>> {
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
impl<'a, M> View<'a, M> {
    pub(crate) fn changed(self) -> impl SortedPairIterator<&'a Id, &'a M, Item = (&'a Id, &'a M)> {
        // Due to Rust's poor type inference on closures, I must write this inline:
        // let changed = |(changed, modify): &(bool, _)| changed.then_some(modify);
        let overlay = self.overlay.into_iter().flatten();
        let overlay = overlay.filter_map_values(|(c, m)| c.then(|| m));
        let root = self.root.iter().filter_map_values(|(c, m)| c.then(|| m));

        overlay.outer_join(root).filter_map_values(|(l, r)| l.or(r))
    }
    pub fn get(&self, id: Id) -> Option<&'a M> {
        self.overlay
            .and_then(|btm| btm.get(&id))
            .or_else(|| self.root.get(&id))
            .map(|(_c, modify)| modify)
    }
}
