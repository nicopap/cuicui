use std::fmt;

use super::{MakeModify, Modify, ModifyIndex, ModifyKind};

// Manual `impl` because we don't want `MakeModify: Debug where M: Debug`, only
// `MakeModify: Debug where M::Item: Debug, PrefabField<M>: Debug`
impl<M: Modify> fmt::Debug for MakeModify<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Mk")
            .field(&self.kind)
            .field(&self.range)
            .finish()
    }
}
// Manual `impl` because we don't want `ModifyKind: Debug where M: Debug`, only
// `ModifyKind: Debug where M::Item: Debug, PrefabField<M>: Debug`
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
