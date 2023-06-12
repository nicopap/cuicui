use fab::binding;

use crate::{tree, RuntimeFormat};

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Format {
    UserDefined(binding::Id),
    Fmt(RuntimeFormat),
}
impl Format {
    fn from_tree<M>(bindings: &mut binding::World<M>, tree: tree::Format) -> Self {
        match tree {
            tree::Format::UserDefined(name) => Format::UserDefined(bindings.get_or_add(name)),
            tree::Format::Fmt(fmt) => Format::Fmt(fmt),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Hook<'a> {
    // TODO: use binding::id also for Source
    pub source: tree::Source<'a>,
    pub format: Option<Format>,
}
impl<'a> Hook<'a> {
    pub(crate) fn from_tree<M>(
        bindings: &mut binding::World<M>,
        binding: tree::Binding<'a>,
    ) -> Option<Self> {
        if let tree::Path::Tracked(source) = binding.path {
            let format = binding.format.map(|f| Format::from_tree(bindings, f));
            Some(Hook { source, format })
        } else {
            None
        }
    }
}
