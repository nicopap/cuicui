//! Global world-scopped data relevant to [`BevyModify`]s located in the bevy ECS.

use std::sync::Arc;

use bevy::prelude::{error, Mut, Reflect, Resource, World};
use fab::binding::{self, Entry};
use fab_parse::{tree::Hook as ParsedHook, Styleable};
use log::warn;
use thiserror::Error;

use crate::track::{GetError, ParseError, Read, UserWrites, Write, WriteError};
use crate::BevyModify;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Get(#[from] GetError),
    #[error(transparent)]
    Write(#[from] WriteError),
}

/// A function that adds styles to the parsed format strings.
///
/// See [`Styles`] documentation for a detailed breakdown on how to use this
/// to its full potential.
pub type StyleFn<M> = Box<dyn FnMut(Styleable<'_, M>) -> Styleable<'_, M> + Send + Sync + 'static>;

/// Stores the styles used when parsing format strings.
///
/// Styles are transformation operations on the parse tree of the format strings.
/// Check out the [`Styleable`] methods for an exhaustive list of what is
/// possible.
///
/// # Style transform operations
///
/// Transform operations currently only include `alias` and `chop`.
///
/// ## `alias`
///
/// `alias` let you translate a modifier
/// (the thingies to the left of the `|` in sections `{zoo: bee|whaboo}`)
/// into another set of modifiers (this can be **several** modifiers).
///
/// There are several ways of defining aliases. Either you can go ham and stringly
/// typed, and only manipulate text (the parsing will fail later if you generate invalid
/// code), or you directly return concrete modifier values.
///
/// Since a modifier always has a string value, you can even parametrize the
/// resulting modifiers on that value. See the [`Styleable::alias`]
/// documentation for details.
///
/// ## `chop`
///
/// `chop` operations let you designate a modifier name to use to **split**
/// sections either _by word_ or _by character_. Then insert an individual
/// modifier in each chopped section.
///
/// All pre-existing modifiers are extended to work on the split sections transparently.
/// Even subsections of the section with the `chop` modifier will be correctly handled.
/// It's honestly amazing this is even possible on a theoretical level.
///
/// The `chop` methods on [`Styleable`] always accept a `FnMut` that returns
/// a modifier. This `FnMut` will be called several times — once per created section.
/// You should return a different one per call.
///
/// For example, if you want to define a `Rainbow` modifier, you'd return a `HueShift`
/// modifier with a different amount of shift per section.
///
/// [`Styleable`] provides methods to make this a bit less error prone.
/// Relying on mutable state in a `FnMut` is always a bit tricky.
#[derive(Resource)]
pub struct Styles<M> {
    styles: Vec<StyleFn<M>>,
}
impl<M: BevyModify> Styles<M> {
    /// Apply all the styles to the `transform`
    pub(crate) fn process<'a>(&mut self, transform: Styleable<'a, M>) -> Styleable<'a, M> {
        self.styles.iter_mut().fold(transform, |acc, f| f(acc))
    }
    /// Insert a new style before all others.
    pub fn overwrite(
        &mut self,
        style: impl FnMut(Styleable<M>) -> Styleable<M> + Send + Sync + 'static,
    ) {
        self.styles.insert(0, Box::new(style));
    }
    /// Add a new style after existing ones.
    pub fn add(&mut self, style: impl FnMut(Styleable<M>) -> Styleable<M> + Send + Sync + 'static) {
        self.styles.push(Box::new(style));
    }
    pub fn new(style: impl FnMut(Styleable<M>) -> Styleable<M> + Send + Sync + 'static) -> Self {
        Styles { styles: vec![Box::new(style)] }
    }
}
impl<M> Default for Styles<M> {
    fn default() -> Self {
        Styles { styles: Vec::new() }
    }
}

/// A hook from a value in the ECS to a [`M: Modify`] associated with
/// a binding.
///
/// In the format string, a hook is a special binding that declares a value
/// to read from the ECS and how to interpret it.
///
/// This typically looks like `{Res.ResourceType.field.to.value:formatting}`.
///
/// All hooks are added the the [`WorldBindings`] resource.
///
/// [`M: Modify`]: fab::modify::Modify
pub struct Hook<M> {
    binding: binding::Id,
    read: Read,
    write: Write<M>,
}
impl<M: BevyModify> Hook<M> {
    fn from_parsed(
        hook: ParsedHook,
        world: &mut World,
        writes: &UserWrites<M>,
        intern: impl FnOnce(&str) -> binding::Id,
    ) -> Result<Self, Error> {
        Ok(Hook {
            binding: intern(hook.source.binding),
            read: Read::from_parsed(hook.source, world)?,
            write: Write::from_parsed(hook.format, writes)?,
        })
    }

    /// Read value describe in `self.read` from [`World`],
    /// then write it into binding `self.binding` in [`WorldBindings`]
    /// according to `self.write`.
    ///
    /// Note: `self` is mutable here, this is because [`Read`] caches world
    /// access to later access the value it reads much faster.
    fn read_into_binding(
        &mut self,
        world: &World,
        bindings: &mut binding::World<M>,
    ) -> Result<(), Error> {
        let value = self.read.world(world)?;
        self.write.modify(value, bindings.entry(self.binding));
        Ok(())
    }
}

/// The binding for all [`M: BevyModify`] in the ECS, and the hooks used by those.
///
/// In the format string, a hook is a special binding that declares a value
/// to read from the ECS and how to interpret it.
///
/// Hooks are added to this resource by [`parse_into_resolver_system`] and read by
/// [`update_hooked`] to update [`WorldBindings`] with the content of hooked values.
///
/// [`parse_into_resolver_system`]: crate::make::parse_into_resolver_system
/// [`M: BevyModify`]: BevyModify
#[derive(Resource)]
pub struct WorldBindings<M> {
    pub bindings: binding::World<M>,
    hooks: Vec<Hook<M>>,
    formatters: UserWrites<M>,
}
impl<M> Default for WorldBindings<M> {
    fn default() -> Self {
        WorldBindings {
            bindings: Default::default(),
            hooks: Vec::new(),
            formatters: UserWrites::new(),
        }
    }
}
impl<M: BevyModify> WorldBindings<M> {
    pub fn add_formatter<T: Reflect>(
        &mut self,
        name: impl Into<String>,
        formatter: impl Fn(&T, Entry<M>) + Send + Sync + 'static,
    ) {
        self.add_reflect_formatter(name, move |reflect, e| {
            let Some(value) = reflect.downcast_ref() else { return; };
            formatter(value, e);
        })
    }
    pub fn add_reflect_formatter(
        &mut self,
        name: impl Into<String>,
        value: impl Fn(&dyn Reflect, Entry<M>) + Send + Sync + 'static,
    ) {
        self.formatters.insert(name.into(), Arc::new(value));
    }
    pub fn add_hooks(&mut self, iter: impl IntoIterator<Item = Hook<M>>) {
        self.hooks.extend(iter)
    }
    pub fn parse_hook(&mut self, hook: ParsedHook, world: &mut World) {
        let Self { bindings, hooks, formatters } = self;
        match Hook::from_parsed(hook, world, formatters, |n| bindings.get_or_add(n)) {
            Ok(hook) => hooks.push(hook),
            Err(err) => error!("A tracker failed to be loaded: {err}"),
        }
    }
}

/// Update [`M::Items`] components co-located with [`LocalBindings`] that declare
/// a reflection-based dependency (`Res.foo.bar`, `One(MarkerComp).path`, etc)
/// when that dependency is updated.
///
/// [`M::Items`]: fab::modify::Modify::Items
/// [`LocalBindings`]: crate::LocalBindings
pub fn update_hooked<M: BevyModify>(world: &mut World) {
    world.resource_scope(|world, mut bindings: Mut<WorldBindings<M>>| {
        let WorldBindings { bindings, hooks, .. } = &mut *bindings;
        for hook in hooks.iter_mut() {
            if let Err(err) = hook.read_into_binding(world, bindings) {
                warn!("Error while running binding: {err}");
            }
        }
    })
}
