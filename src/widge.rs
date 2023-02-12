pub mod action_button;
// pub mod checkbox;
pub mod event_button;
pub mod labelled;
pub mod list;
pub mod pick;
pub mod visual;

use crate::{Prefab, WorldValue};

/// A value that has a `cuicui` representation.
///
/// It supports spawning a `Prefab`
pub trait Widge: Prefab + WorldValue {}
