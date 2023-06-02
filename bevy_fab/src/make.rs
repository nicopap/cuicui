use std::{fmt, marker::PhantomData, mem};

use bevy::ecs::{prelude::*, system::SystemState};
use fab::prefab::{FieldsOf, PrefabContext, PrefabField};
use fab::resolve::Resolver;
use fab_parse::tree as parse;
use log::error;

use crate::{
    track::{Hook, Hooks},
    BevyPrefab, PrefabLocal, PrefabWorld,
};

#[derive(Component)]
pub struct ParseFormatString<P: BevyPrefab> {
    pub format_string: String,
    pub default_item: P::Item,
    pub items_extra: Option<P::ItemsCtorData>,
    _p: PhantomData<fn(P)>,
}
impl<P: BevyPrefab> ParseFormatString<P> {
    pub fn new(
        format_string: String,
        default_item: P::Item,
        items_extra: P::ItemsCtorData,
    ) -> Self {
        let _p = PhantomData;
        let items_extra = Some(items_extra);
        Self { format_string, default_item, items_extra, _p }
    }
    /// Drain all fields from a `&mut Self` to get an owned value.
    fn take(&mut self) -> (P::ItemsCtorData, P::Item, String)
    where
        P::Item: Clone,
    {
        (
            self.items_extra.take().unwrap(),
            self.default_item.clone(),
            mem::take(&mut self.format_string),
        )
    }
}

/// Create a [`Resolver`] by parsing `format_string`.
///
/// Effects:
///
/// - Returns `Vec<BP::Item>`: The list of items the [`Resolver`] will work on.
/// - Returns `Resolver<BP, R>`: The resolver containing the parsed [`Modify`].
/// - Returns `Vec<parse::Hook<'fstr>>`: The parsed but not created [`Hook`]s used in
///   the format string. It has the lifetime of `format_string`.
/// - Interns in [`PrefabWorld<BP>`] bindings found in `format_string`.
fn mk<'fstr, BP: BevyPrefab, const R: usize>(
    bindings: &mut PrefabWorld<BP>,
    default_item: &BP::Item,
    context: &PrefabContext<BP>,
    format_string: &'fstr str,
) -> anyhow::Result<(Vec<BP::Item>, Resolver<BP, R>, Vec<parse::Hook<'fstr>>)>
where
    BP::Items: Component,
    BP::Item: Clone + fmt::Debug,
    PrefabField<BP>: fmt::Debug,
{
    let mut new_hooks = Vec::new();

    let tree = fab_parse::format_string(format_string)?;
    let tree = BP::transform(tree.transform());
    let parsed = tree.finish(&mut bindings.0, &mut new_hooks);
    let parsed: Vec<_> = parsed.into_iter().collect::<anyhow::Result<_>>()?;

    let (resolver, items) = Resolver::new(parsed, default_item, context);

    Ok((items, resolver, new_hooks))
}

/// Replaces [`ParseFormatString`] with [`PrefabLocal`],
/// updating [`PrefabWorld<BP>`] and [`Hooks`].
///
/// This is an exclusive system, as it requires access to the [`World`] to generate
/// the [`Hook`]s specified in the format string.
pub fn parse_into_resolver_system<BP: BevyPrefab + 'static, const R: usize>(
    world: &mut World,
    mut to_make: Local<QueryState<(Entity, &mut ParseFormatString<BP>)>>,
    mut cache: Local<SystemState<(Commands, ResMut<PrefabWorld<BP>>, BP::Param)>>,
) where
    BP::Items: Component,
    BP::Item: Clone + fmt::Debug,
    PrefabField<BP>: fmt::Debug,
    BP::Modify: fmt::Write + From<String>,
    FieldsOf<BP>: Sync + Send,
{
    // The `format_string` are field of `ParseFormatString`, components of the ECS.
    // we use `ParseFormatString::take` to extract them from the ECS, and own them
    // in this system in `to_make`.
    let to_make: Vec<_> = to_make
        .iter_mut(world)
        .map(|(e, mut r)| (e, r.take()))
        .collect();

    if to_make.is_empty() {
        return;
    }

    // The `parse::Hook`s returned by `mk`
    // have a lifetime dependent on the `format_string` used.
    //
    // parse::Hook's reference here points to String within MakeRichText in
    // the `to_make` variable.
    let mut new_hooks: Vec<_> = Vec::new();

    // Furthermore, `richtext::mk` needs mutable access to WorldBindings and
    // immutable to the context, so we use the SystemState to extract them.
    {
        let (mut cmds, mut world_bindings, params) = cache.get_mut(world);

        let context = BP::context(&params);

        // TODO(perf): batch commands update.
        for (entity, (ctor_data, default_item, format_string)) in to_make.iter() {
            match mk::<_, R>(&mut world_bindings, default_item, &context, format_string) {
                Ok((items, resolver, mut hooks)) => {
                    new_hooks.append(&mut hooks);

                    let local = PrefabLocal::new(resolver, default_item.clone());
                    let items = BP::make_items(ctor_data, items);

                    cmds.entity(*entity)
                        .insert((local, items))
                        .remove::<ParseFormatString<BP>>();
                }
                Err(err) => {
                    error!("Error when building a resolver: {err}");
                }
            }
        }
    }
    cache.apply(world);

    // To convert the parse::Hook into an actual track::Hook that goes into track::Hooks,
    // we need excluisve world access.
    world.resource_scope(|world, mut hooks: Mut<Hooks<BP::Modify>>| {
        world.resource_scope(|world, mut bindings: Mut<PrefabWorld<BP>>| {
            new_hooks.iter().for_each(|hook| {
                if let Some(hook) = Hook::from_parsed(*hook, world, |n| bindings.0.get_or_add(n)) {
                    hooks.extend(Some(hook));
                } else {
                    error!("A tracker failed to be loaded");
                }
            });
        });
    });
}
