//! Stores [`Modify`].

use std::{collections::BTreeMap, mem};

use anyhow::anyhow;
use smallvec::SmallVec;
use string_interner::{backend::StringBackend, StringInterner, Symbol};

use crate::{
    joined_sort::joined_sort, joined_sort::Ior, modify::BindingId, AnyError, IntoModify, Modify,
    ModifyBox,
};

#[derive(Debug, Default)]
pub struct LocalBindings {
    inner: BTreeMap<BindingId, (bool, ModifyBox)>,
    buffered: Vec<(Box<str>, ModifyBox)>,
    resolved: SmallVec<[(Box<str>, BindingId); 2]>,
}
#[derive(Debug)]
pub struct WorldBindings {
    inner: BTreeMap<BindingId, (bool, ModifyBox)>,
    interner: StringInterner<StringBackend<BindingId>>,
}
impl Default for WorldBindings {
    fn default() -> Self {
        WorldBindings {
            inner: BTreeMap::default(),
            interner: StringInterner::new(),
        }
    }
}
#[derive(Clone, Copy)]
pub struct BindingsView<'a> {
    root: &'a BTreeMap<BindingId, (bool, ModifyBox)>,
    overlay: Option<&'a BTreeMap<BindingId, (bool, ModifyBox)>>,
}

impl LocalBindings {
    pub fn set_by_id(&mut self, id: BindingId, value: impl IntoModify) {
        self.inner.insert(id, (true, value.into_modify()));
    }
    pub fn set(&mut self, binding_name: impl Into<String>, value: impl IntoModify) {
        let name = binding_name.into();
        let is_name = |(known, _): &(Box<str>, _)| known.as_ref().cmp(&name);
        if let Ok(resolved) = self.resolved.binary_search_by(is_name) {
            let id = self.resolved[resolved].1;
            self.inner.insert(id, (true, value.into_modify()));
        } else {
            self.buffered.push((name.into(), value.into_modify()));
        }
    }
    pub fn sync(&mut self, global: &WorldBindings) -> Result<(), AnyError> {
        let LocalBindings { buffered, inner, resolved } = self;
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
        self.inner.values_mut().for_each(|v| v.0 = false);
    }
}
impl WorldBindings {
    // TODO(err): Should return Result
    /// Set a named modifier binding.
    ///
    /// Returns `None` if the `key` has no binding.
    ///
    /// Unlike [`RichTextData`] this doesn't check that the key exists or that
    /// `value` is of the right type.
    pub fn set(&mut self, key: &str, value: impl IntoModify) -> Option<()> {
        let id = self.interner.get(key)?;
        self.set_id(id, value);
        Some(())
    }
    pub fn set_id(&mut self, id: BindingId, value: impl IntoModify) {
        match self.inner.get_mut(&id) {
            Some(old_value) => {
                let value = value.into_modify();
                if !old_value.1.eq_dyn(value.as_ref()) {
                    *old_value = (true, value);
                }
            }
            None => {
                self.inner.insert(id, (true, value.into_modify()));
            }
        }
    }
    pub(crate) fn get_or_add(&mut self, name: impl AsRef<str>) -> BindingId {
        self.interner.get_or_intern(name)
    }
    pub fn view(&self) -> BindingsView {
        BindingsView { root: &self.inner, overlay: None }
    }
    pub fn view_with_local<'a>(
        &'a self,
        local: &'a mut LocalBindings,
    ) -> Result<BindingsView<'a>, AnyError> {
        local.sync(self)?;
        Ok(BindingsView { overlay: Some(&local.inner), root: &self.inner })
    }
}
impl<'a> BindingsView<'a> {
    pub(crate) fn changed(&self) -> impl Iterator<Item = (BindingId, &ModifyBox)> + '_ {
        // Due to Rust's poor type inference on closures, I must write this inline
        // let only_updated = |(id, (changed, modify))| changed.then_some((*id, modify));
        let overlay = self
            .overlay
            .iter()
            .flat_map(|b| *b)
            .filter_map(|(id, (changed, m))| changed.then_some((*id, m)));
        let root = self
            .root
            .iter()
            .filter_map(|(id, (changed, m))| changed.then_some((*id, m)));

        joined_sort(overlay, root, |l, r| l.0.cmp(&r.0)).map(Ior::prefer_left)
    }
    pub fn get(&self, id: BindingId) -> Option<&'a (dyn Modify + Send + Sync)> {
        self.overlay
            .and_then(|btm| btm.get(&id))
            .or_else(|| self.root.get(&id))
            .map(|(_c, modify)| modify.as_ref())
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
