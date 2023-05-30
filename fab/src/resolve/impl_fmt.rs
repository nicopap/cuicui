use std::fmt;

use super::{FieldsOf, MakeModify, Modifier, ModifyIndex, ModifyKind, Prefab};

// Manual `impl` because we don't want `Modifier: Debug where P: Debug`, only
// `Modifier: Debug where P::Item: Debug, PrefabField<P>: Debug`
impl<P: Prefab> fmt::Debug for Modifier<P>
where
    P::Modify: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Modifier")
            .field("inner", &self.modify)
            .field("range", &self.range)
            .finish()
    }
}
// Manual `impl` because we don't want `MakeModify: Debug where P: Debug`, only
// `MakeModify: Debug where P::Item: Debug, PrefabField<P>: Debug`
impl<P: Prefab> fmt::Debug for MakeModify<P>
where
    P::Modify: fmt::Debug,
    FieldsOf<P>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Mk")
            .field(&self.kind)
            .field(&self.range)
            .finish()
    }
}
// Manual `impl` because we don't want `ModifyKind: Debug where P: Debug`, only
// `ModifyKind: Debug where P::Item: Debug, PrefabField<P>: Debug`
impl<P: Prefab> fmt::Debug for ModifyKind<P>
where
    P::Modify: fmt::Debug,
    FieldsOf<P>: fmt::Debug,
{
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
