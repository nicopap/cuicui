//! Custom `Debug` impl for `resolver` structs, so that debug print output
//! is more dense and easier to parse as a human.
use std::fmt;

use super::{MakeModify, Modifier, Modify, ModifyIndex, ModifyKind};

impl<M: Modify> fmt::Debug for MakeModify<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Mk")
            .field(&self.kind)
            .field(&self.range)
            .finish()
    }
}
impl<M: Modify> fmt::Debug for ModifyKind<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModifyKind::Bound { binding, .. } => f.debug_tuple("Bound").field(binding).finish(),
            ModifyKind::Modify(modify) => f.debug_tuple("Set").field(&modify).finish(),
        }
    }
}
impl fmt::Debug for ModifyIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<M{}>", self.0)
    }
}

impl<M: fmt::Debug> fmt::Debug for Modifier<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Mod")
            .field(&self.modify)
            .field(&self.range)
            .finish()
    }
}
