use std::{marker::PhantomData, mem};

use bevy::ecs::{prelude::*, system::SystemState};
use fab::{modify::FieldsOf, resolve::Resolver};
use fab_parse::tree as parse;
use log::error;

use crate::{BevyModify, LocalBindings, Styles, WorldBindings};

#[derive(Component)]
pub struct ParseFormatString<BM: BevyModify> {
    pub format_string: String,
    pub default_item: BM::Item,
    pub items_extra: Option<BM::ItemsCtorData>,
    _p: PhantomData<fn(BM)>,
}
impl<P: BevyModify> ParseFormatString<P> {
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
    fn take(&mut self) -> (P::ItemsCtorData, P::Item, String) {
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
/// - Returns `Vec<BM::Item>`: The list of items the [`Resolver`] will work on.
/// - Returns `Resolver<BM, R>`: The resolver containing the parsed [`Modify`].
/// - Returns `Vec<parse::Hook<'fstr>>`: The parsed but not created [`Hook`]s used in
///   the format string. It has the lifetime of `format_string`.
/// - Interns in [`WorldBindings<BM>`] bindings found in `format_string`.
fn mk<'fstr, BM: BevyModify>(
    bindings: &mut WorldBindings<BM>,
    style: &Styles<BM>,
    default_item: &BM::Item,
    context: &BM::Context<'_>,
    format_string: &'fstr str,
) -> anyhow::Result<(Vec<BM::Item>, BM::Resolver, Vec<parse::Hook<'fstr>>)>
where
    BM::Items: Component,
{
    let mut new_hooks = Vec::new();

    let tree = fab_parse::format_string(format_string)?;
    let tree = style.process(tree.transform());
    let parsed = tree.finish(&mut bindings.bindings, &mut new_hooks);
    let parsed: Vec<_> = parsed.into_iter().collect::<anyhow::Result<_>>()?;

    let (resolver, items) = BM::Resolver::new(parsed, default_item, context);

    Ok((items, resolver, new_hooks))
}

/// Replaces [`ParseFormatString`] with [`LocalBindings`],
/// updating [`WorldBindings<BM>`].
///
/// This is an exclusive system, as it requires access to the [`World`] to generate
/// the [`Hook`]s specified in the format string.
pub fn parse_into_resolver_system<BM: BevyModify + 'static>(
    world: &mut World,
    mut to_make: Local<QueryState<(Entity, &mut ParseFormatString<BM>)>>,
    mut cache: Local<
        SystemState<(
            Commands,
            Res<Styles<BM>>,
            ResMut<WorldBindings<BM>>,
            BM::Param,
        )>,
    >,
) where
    BM::Items: Component,
    FieldsOf<BM>: Sync + Send,
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
        let (mut cmds, styles, mut world_bindings, params) = cache.get_mut(world);

        let context = BM::context(&params);

        // TODO(perf): batch commands update.
        for (entity, (ctor_data, default_item, format_string)) in to_make.iter() {
            match mk(
                &mut world_bindings,
                &styles,
                default_item,
                &context,
                format_string,
            ) {
                Ok((items, resolver, mut hooks)) => {
                    new_hooks.append(&mut hooks);

                    let local = LocalBindings::<BM>::new(resolver, default_item.clone());
                    let items = BM::make_items(ctor_data, items);

                    cmds.entity(*entity)
                        .insert((local, items))
                        .remove::<ParseFormatString<BM>>();
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
    world.resource_scope(|world, mut bindings: Mut<WorldBindings<BM>>| {
        let parse_hook = |&hook| bindings.parse_hook(hook, world);
        new_hooks.iter().for_each(parse_hook);
    });
}
