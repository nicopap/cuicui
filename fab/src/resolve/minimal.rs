//! A minimal resolver that only support binding to individual sections
//! and ignores all other features.

use log::{error, warn};
use nonmax::NonMaxU32;

use super::{MakeModify, ModifyKind, Resolver};
use crate::binding::View;
use crate::modify::{Changing, Indexed, Modify};

/// A resolver with minimal overhead and functionalities.
///
/// What this can do:
/// - Generate a default `Vec<Item>` based on static modifiers in `modifiers`
/// - Store and trigger bindings that modify a single sections. Unlike [`DepsResolver`]
///   this is a simple array lookup.
///   
/// [`DepsResolver`]: super::DepsResolver
pub struct MinResolver {
    indices: Box<[Option<NonMaxU32>]>,
}
impl<M: Modify> Resolver<M> for MinResolver {
    fn new(
        modifiers: Vec<MakeModify<M>>,
        default_section: &<M as Modify>::Item,
        ctx: &<M as Modify>::Context<'_>,
    ) -> (Self, Vec<<M as Modify>::Item>) {
        let warn_range = "Skipping bindings touching more than a single sections in MinResolver.";

        let Some(section_count) = modifiers.iter().map(|m| m.range.end).max() else {
            return (MinResolver { indices: Box::new([])}, vec![])
        };
        let mut sections = vec![default_section.clone(); section_count as usize];
        let mut bindings = Vec::new();

        for modifier in modifiers.into_iter() {
            match modifier.kind {
                ModifyKind::Bound { binding, .. } => {
                    if modifier.range.end != modifier.range.start + 1 {
                        warn!("{warn_range}");
                    } else {
                        bindings.push((binding.0 as usize, modifier.range.start))
                    }
                }
                ModifyKind::Modify(modify) => {
                    let range = modifier.range.start as usize..modifier.range.end as usize;

                    // SAFETY: `sections`'s `len` is `max(range.end)` meaning we are always
                    // within bounds.
                    let sections = unsafe { sections.get_unchecked_mut(range) };

                    sections.iter_mut().for_each(|section| {
                        if let Err(err) = modify.apply(ctx, section) {
                            error!("Error occured when applying modify: {err}");
                        }
                    });
                }
            }
        }
        let mut indices = if let Some(max_binding) = bindings.iter().map(|b| b.0).max() {
            vec![None; max_binding].into_boxed_slice()
        } else {
            return (MinResolver { indices: Box::new([]) }, sections);
        };
        for &(binding, section) in &bindings {
            indices[binding] = Some(NonMaxU32::new(section).unwrap());
        }
        (MinResolver { indices }, sections)
    }

    fn update<'a>(
        &'a self,
        to_update: &mut <M as Modify>::Items,
        _updates: &'a Changing<M>,
        bindings: View<'a, M>,
        ctx: &<M as Modify>::Context<'_>,
    ) {
        bindings.changed().for_each(|(binding, modify)| {
            let Some(Some(index)) = self.indices.get(binding.0 as usize) else { return; };
            let index = index.get() as usize;

            let Some(section) = to_update.get_mut(index) else { return; };
            if let Err(err) = modify.apply(ctx, section) {
                error!("Error occured when applying modify: {err}");
            }
        });
    }
}
