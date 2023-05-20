//! Stores [`Modify`].

use std::{collections::BTreeMap, fmt, mem};

use anyhow::anyhow;
use datazoo::{AssumeSortedByKeyExt, SortedPairIterator};
use smallvec::SmallVec;
use string_interner::{backend::StringBackend, StringInterner, Symbol};

use crate::prefab::Prefab;

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BindingId(pub(crate) u32);

impl fmt::Debug for BindingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<B{}>", self.0)
    }
}

#[derive(Debug, Default)]
pub struct LocalBindings<P: Prefab> {
    bindings: BTreeMap<BindingId, (bool, P::Modifiers)>,
    buffered: Vec<(Box<str>, P::Modifiers)>,
    resolved: SmallVec<[(Box<str>, BindingId); 2]>,
}
#[derive(Debug)]
pub struct WorldBindings<P: Prefab> {
    bindings: BTreeMap<BindingId, (bool, P::Modifiers)>,
    interner: StringInterner<StringBackend<BindingId>>,
}
impl<P: Prefab> Default for WorldBindings<P> {
    fn default() -> Self {
        WorldBindings {
            bindings: BTreeMap::default(),
            interner: StringInterner::new(),
        }
    }
}
#[derive(Clone, Copy)]
pub struct BindingsView<'a, P: Prefab> {
    root: &'a BTreeMap<BindingId, (bool, P::Modifiers)>,
    overlay: Option<&'a BTreeMap<BindingId, (bool, P::Modifiers)>>,
}

impl<P: Prefab> LocalBindings<P> {
    pub fn set_by_id(&mut self, id: BindingId, value: P::Modifiers) {
        self.bindings.insert(id, (true, value));
    }
    pub fn set(&mut self, binding_name: impl Into<String>, value: P::Modifiers) {
        let name = binding_name.into();
        let is_name = |(known, _): &(Box<str>, _)| known.as_ref().cmp(&name);
        if let Ok(resolved) = self.resolved.binary_search_by(is_name) {
            let id = self.resolved[resolved].1;
            self.bindings.insert(id, (true, value));
        } else {
            self.buffered.push((name.into(), value));
        }
    }
    pub fn sync(&mut self, global: &WorldBindings<P>) -> anyhow::Result<()> {
        let LocalBindings { buffered, bindings: inner, resolved } = self;
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
impl<P: Prefab> WorldBindings<P>
where
    P::Modifiers: PartialEq,
{
    // TODO(err): Should return Result
    /// Set a named modifier binding.
    ///
    /// Returns `None` if the `key` has no binding.
    ///
    /// Unlike [`RichTextData`] this doesn't check that the key exists or that
    /// `value` is of the right type.
    pub fn set(&mut self, key: &str, value: P::Modifiers) -> Option<()> {
        let id = self.interner.get(key)?;
        self.set_id(id, value);
        Some(())
    }
    pub fn set_id(&mut self, id: BindingId, value: P::Modifiers) {
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
    pub(crate) fn get_or_add(&mut self, name: impl AsRef<str>) -> BindingId {
        self.interner.get_or_intern(name)
    }
    pub fn view(&self) -> BindingsView<P> {
        BindingsView { root: &self.bindings, overlay: None }
    }
    pub fn view_with_local<'a>(
        &'a self,
        local: &'a mut LocalBindings<P>,
    ) -> anyhow::Result<BindingsView<'a, P>> {
        local.sync(self)?;
        Ok(BindingsView {
            overlay: Some(&local.bindings),
            root: &self.bindings,
        })
    }
}
impl<'a, P: Prefab> BindingsView<'a, P> {
    pub(crate) fn changed(&self) -> impl Iterator<Item = (BindingId, &P::Modifiers)> + '_ {
        // Due to Rust's poor type inference on closures, I must write this inline
        // let only_updated = |(id, (changed, modify))| changed.then_some((*id, modify));
        let overlay = self
            .overlay
            .iter()
            .flat_map(|b| *b)
            .filter_map(|(id, (changed, m))| changed.then_some((*id, m)))
            .assume_sorted_by_key();
        let root = self
            .root
            .iter()
            .filter_map(|(id, (changed, m))| changed.then_some((*id, m)))
            .assume_sorted_by_key();

        overlay.outer_join(root).filter_map_values(|(l, r)| l.or(r))
    }
    pub fn get(&self, id: BindingId) -> Option<&'a P::Modifiers> {
        self.overlay
            .and_then(|btm| btm.get(&id))
            .or_else(|| self.root.get(&id))
            .map(|(_c, modify)| modify)
    }
}

impl Symbol for BindingId {
    fn try_from_usize(index: usize) -> Option<Self> {
        let u32 = u32::try_from(index).ok()?;
        Some(BindingId(u32))
    }

    fn to_usize(self) -> usize {
        assert!(
            mem::size_of::<usize>() >= mem::size_of::<Self>(),
            "NOTE: please open an issue if you need to run bevy on 16 bits plateforms"
        );
        self.0 as usize
    }
}
