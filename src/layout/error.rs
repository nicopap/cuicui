use std::fmt;

use bevy::prelude::{Entity, Name, Query};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Handle {
    Unnamed(Entity),
    Named(Name),
}
impl Handle {
    pub(super) fn of(entity: Entity, names: &Query<&Name>) -> Self {
        match names.get(entity) {
            Ok(name) => Handle::Named(name.clone()),
            Err(_) => Handle::Unnamed(entity),
        }
    }
}
impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Handle::Unnamed(entity) => write!(f, "<{entity:?}>"),
            Handle::Named(name) => write!(f, "{name}"),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Error)]
pub(super) enum Why {
    #[error(
        "{this} needs a parent with a specified {axis}, \
        but {parent}, an ancestor of {this} undefines the size on {axis}."
    )]
    ParentIsStretch {
        this: Handle,
        parent: Handle,
        axis: &'static str,
    },
}
pub(super) fn parent_is_stretch(
    axis: &'static str,
    this: Entity,
    parent: Entity,
    query: &Query<&Name>,
) -> Why {
    Why::ParentIsStretch {
        this: Handle::of(this, query),
        parent: Handle::of(parent, query),
        axis,
    }
}
