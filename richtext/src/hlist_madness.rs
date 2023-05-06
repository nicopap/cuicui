use std::{any::Any, fmt};

use anyhow::anyhow;
use bevy::text::TextSection;

use crate::{modify::Context, AnyError, Modify};

trait List {
    type Tail: List;
    type Head: Modify + Clone + PartialEq + Any + fmt::Debug + Send + Sync + 'static;
    const TERMINAL: bool = false;
}
impl<H, T: List> List for (H, T)
where
    H: Modify + Clone + PartialEq + Any + fmt::Debug + Send + Sync + 'static,
{
    type Tail = T;
    type Head = H;
}
enum Nil {}
impl List for Nil {
    type Tail = Nil;
    type Head = ();
    const TERMINAL: bool = true;
}

impl<H, T> Modify for (H, T)
where
    H: Modify + Clone + PartialEq + Any + fmt::Debug + Send + Sync + 'static,
    T: Modify + Clone + PartialEq + Any + fmt::Debug + Send + Sync + 'static,
{
    fn apply(&self, ctx: &Context, text: &mut TextSection) -> Result<(), AnyError> {
        self.0.apply(ctx, text)?;
        self.1.apply(ctx, text)
    }
    fn clone_dyn(&self) -> super::ModifyBox {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn eq_dyn(&self, other: &dyn Modify) -> bool {
        let any = other.as_any();
        let Some(right) = any.downcast_ref::<Self>() else { return false; };
        self == right
    }
    fn debug_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::fmt::Debug;
        self.fmt(f)
    }
}

pub fn modifiers<MT, ZipL: List, ZipR: List>(
    replay: bool,
    data: &[(&'static str, &str)],
    modify_tail: MT,
) -> Result<Box<dyn Modify>, AnyError>
where
    MT: Modify + Clone + PartialEq + Any + fmt::Debug + Send + Sync + 'static,
{
    if ZipR::TERMINAL && replay {
        return Err(anyhow!("see if I care"));
    }
    match data {
        [] => Ok(Box::new(modify_tail)),
        [(n1, v1), ..] => match () {
            () if ZipR::Head::name() == *n1 => {
                let data_tail = &data[1..];
                let v1_parsed = ZipR::Head::parse(v1)?;
                modifiers::<_, ZipL, ZipR::Tail>(false, data_tail, (v1_parsed, modify_tail))
            }
            () if ZipR::TERMINAL => modifiers::<_, Nil, ZipL>(true, data, modify_tail),
            () => modifiers::<_, (ZipR::Head, ZipL), ZipR::Tail>(replay, data, modify_tail),
        },
    }
}
