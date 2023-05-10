//! Stores [`Modify`].

use std::{collections::BTreeMap, fmt, mem, ops::Range};

use anyhow::anyhow;
use bevy::{reflect::Reflect, reflect::Typed, text::Text, utils::HashMap};
use smallvec::SmallVec;
use string_interner::{backend::StringBackend, StringInterner, Symbol};

use crate::{
    joined_sort::{joined_sort, Ior},
    modify::{self, BindingId, DependsOn},
    parse::{self, interpret},
    show::{self, ShowBox},
    track::Tracker,
    AnyError, IntoModify, Modify, ModifyBox,
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
    pub fn richtext_builder(&mut self, format_string: impl Into<String>) -> RichTextBuilder {
        RichTextBuilder {
            format_string: format_string.into(),
            context: interpret::Context::new(self).with_defaults(),
            formatters: HashMap::default(),
        }
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
    pub(crate) fn changed(&self) -> impl Iterator<Item = BindingId> + '_ {
        let id_changed = |(id, (changed, _)): (&_, &(bool, _))| changed.then_some(*id);
        let overlay = self.overlay.iter().flat_map(|b| *b).filter_map(id_changed);
        let root = self.root.iter().filter_map(id_changed);

        joined_sort(overlay, root, Ord::cmp).map(Ior::prefer_left)
    }
    pub fn get(&self, id: BindingId) -> Option<&'a (dyn Modify + Send + Sync)> {
        self.overlay
            .and_then(|btm| btm.get(&id))
            .or_else(|| self.root.get(&id))
            .map(|(_c, modify)| modify.as_ref())
    }
}

#[derive(Debug)]
pub struct RichText {
    modifiers: Modifiers,

    /// What needs to be triggered when given dependency is updated?
    ///
    /// This list must be sorted in ascending `Dependecy.on` order always.
    dependencies: Box<[Dependency]>,
}

#[derive(Debug)]
pub(crate) struct Modifiers(pub(crate) Box<[Modifier]>);

/// A [`ModifyBox`] that apply to a given [`Range`] of [`TextSection`]s on a [`Text`].
#[derive(Debug)]
pub(crate) struct Modifier {
    /// The modifier to apply in the given `range`.
    pub(crate) modify: ModifyBox,

    /// The range to which to apply the `modify`.
    pub(crate) range: Range<u32>,
}

#[derive(Debug)]
struct Dependency {
    /// What triggers this dependency.
    on: DependsOn,

    /// Indicies of `Modifier` in `Modifiers` that depends on `on`.
    ///
    /// The indicies are in ascending order, so when applying `Modify` on change,
    /// the most general apply before the more specific.
    targets: SmallVec<[u16; 8]>,
}
impl RichText {
    pub fn update<'a>(&'a self, to_update: &'a mut Text, ctx: &'a modify::Context) {
        RichTextCtx { rich: self, ctx, to_update }.update()
    }
    pub fn default_text(&self, ctx: &modify::Context) -> Text {
        let modifiers = self.modifiers.0.iter();
        let section_count = modifiers.map(|m| m.range.end).max().unwrap_or(0);
        let section_count = usize::try_from(section_count).unwrap();
        let mut text = Text {
            sections: vec![Default::default(); section_count],
            ..Default::default()
        };
        self.modifiers.update_all(ctx, &mut text);
        text
    }
}
impl Modifiers {
    fn dependencies(&self) -> Box<[Dependency]> {
        let mut dependencies = BTreeMap::new();
        let get_deps =
            |(i, m): (_, &Modifier)| m.modify.depends_on().into_iter().map(move |d| (i, d));
        let iter = self.0.iter().enumerate().flat_map(get_deps);
        for (i, dep) in iter {
            dependencies
                .entry(dep)
                .or_insert_with(SmallVec::new)
                .push(u16::try_from(i).unwrap());
        }
        dependencies
            .into_iter()
            .map(|(on, targets)| Dependency { on, targets })
            .collect()
    }
    fn update_all(&self, ctx: &modify::Context, text: &mut Text) {
        for modifier in self.0.iter() {
            modifier.update(ctx, text);
        }
    }
}
impl Modifier {
    fn update(&self, ctx: &modify::Context, text: &mut Text) {
        for section in self.range.clone() {
            let section = usize::try_from(section).unwrap();
            // TODO(err): Text should have same size as RichText
            if let Some(to_update) = text.sections.get_mut(section) {
                // TODO(err) :^)
                self.modify.apply(ctx, to_update).unwrap()
            }
        }
    }
}

pub struct RichTextBuilder<'a> {
    format_string: String,
    context: interpret::Context<'a>,
    // TODO(perf): This sucks, the `FetchBox`, which we are using this for, is
    // calling itself the `ShowBox` impl. Instead of storing formatters, we should
    // directly construct the `FetchBox` when it is added
    // TODO(feat): This is actually unused.
    formatters: HashMap<&'static str, ShowBox>,
}
struct RichTextCtx<'a> {
    rich: &'a RichText,
    ctx: &'a modify::Context<'a>,
    to_update: &'a mut Text,
}
impl<'a> RichTextBuilder<'a> {
    /// Add a [formatter](crate::show::Show).
    pub fn fmt<I, O, F>(mut self, name: &'static str, convert: F) -> Self
    where
        I: Reflect + Typed,
        O: fmt::Display + 'static, // TODO(bug): shouldn't need this + 'static
        F: Clone + Send + Sync + Fn(&I) -> O + 'static,
    {
        self.formatters
            .insert(name, show::Convert::<I, O, F>::new(convert));
        self
    }
    pub fn build(self) -> Result<(RichText, Vec<Tracker>), AnyError> {
        let Self { format_string, context, .. } = self;
        let mut trackers = Vec::new();
        let modifiers = parse::richtext(context, &format_string, &mut trackers)?;
        let dependencies = modifiers.dependencies();

        // debug!("Making RichText: {format_string:?}");
        // partial.print_bindings();

        Ok((RichText { modifiers, dependencies }, trackers))
    }
}
impl<'a> RichTextCtx<'a> {
    fn update(self) {
        let triggers = self.ctx.changed();
        self.trigger(triggers);
    }

    /// Updates `to_update` based on `rich` given updated dependencies in `changed`.
    ///
    /// `changed` MUST be sorted and unique, or mayhem will ensue;
    /// Not all relevant `Modify` will be triggered.
    fn trigger(self, changed: impl Iterator<Item = DependsOn>) {
        // TODO(perf):
        // `trigger` is called once per `RichText`, with the 1st iteration scheme,
        // we iterate over `changed` N times, N be how many `RichText` we have.
        // With the 2nd iteration scheme, we iterate D-times which is the total number
        // of dependencies.
        // Depending on the change pattern, one can be better than the other
        // NOTE: Assuming `changed` is sorted, we keep track in `rich_dependency_index`
        // of position in `rich.dependencies`. We check the dependecy definition
        // at this index. Then one of three things happen:
        // 1. dependency = changed: we trigger it, pass to the next dependency and changed
        // 2. dependency < changed: We know the dependency we are looking for is at least _after_
        //    `rich_dependency_index`, so we increment it and look at the next one
        // 3. dependency > changed: changed is not one of our dependency, so we pass to the
        //    next changed
        let mut rich_dependency_index = 0;
        for changed in changed {
            'lööp: loop {
                let Some(depends) = self.rich.dependencies.get(rich_dependency_index) else {
                    // TODO(err): when is this triggered? What error message?
                    return;
                };
                if depends.on > changed {
                    break 'lööp;
                }
                rich_dependency_index += 1;

                if depends.on == changed {
                    for modifier_index in &depends.targets {
                        let modifier_index = usize::from(*modifier_index);
                        let Some(modifier) = self.rich.modifiers.0.get(modifier_index) else {
                            panic!("RichText.dependencies out of sync with RichText.modifiers");
                        };
                        // TODO(bug): Once X is updated, if there is sub-section that depends on X,
                        // they should be updated as well.
                        modifier.update(self.ctx, self.to_update);
                    }
                    break 'lööp;
                }
            }
        }
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
