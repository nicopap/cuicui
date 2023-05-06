//! Get a string from reflection.
use std::{fmt, marker::PhantomData};

use bevy::{
    log::trace,
    math::*,
    reflect::{FromReflect, Reflect, ReflectFromReflect, Typed},
    utils::get_short_name,
};
use thiserror::Error;

pub type ShowBox = Box<dyn Show + Send + Sync + 'static>;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Format(#[from] fmt::Error),
    #[error("Erroneous type. Expected {expected}, got {actual}")]
    BadType { expected: String, actual: String },
    #[error("User defined error: {0}")]
    User(Box<dyn std::error::Error>),
}
impl Error {
    fn from_reflect<E: Typed>(actual: &dyn Reflect) -> Self {
        Error::BadType {
            expected: get_short_name(E::type_info().type_name()),
            actual: get_short_name(actual.type_name()),
        }
    }
}

// TODO(doc)
/// Formatter to interpret pull values.
pub trait Show {
    fn format(&self, input: &dyn Reflect, f: &mut fmt::Formatter<'_>) -> Result<(), Error>;
    fn dyn_clone(&self) -> ShowBox;
}
impl dyn Show {
    /// Implement [`fmt::Display`] on a [`&dyn Reflect`].
    pub fn display<'a>(&'a self, reflect: &'a dyn Reflect) -> ReflectDisplay<'a> {
        ReflectDisplay { show: self, reflect }
    }
}

// TODO(doc): it's about the format string here, describe which types this works
// with, what happens when not in list.
/// "Last chance" [`Show`].
#[derive(Reflect, PartialEq, Debug, Clone, Copy, FromReflect)]
#[reflect(FromReflect)]
pub struct RuntimeFormat {
    pub width: usize,
    pub prec: usize,
    pub sign: bool,
    pub debug: bool,
}
impl Show for RuntimeFormat {
    fn format(&self, input: &dyn Reflect, f: &mut fmt::Formatter<'_>) -> Result<(), Error> {
        macro_rules! try_format_tys {
            (@format $v:ident) => {
                match (self.sign, self.debug) {
                    (true, false) => write!(f, "{:+0w$.p$}", *$v, w = self.width, p = self.prec)?,
                    (false, false) => write!(f, "{:0w$.p$}", *$v, w = self.width, p = self.prec)?,
                    (true, true) => write!(f, "{:+0w$.p$?}", *$v, w = self.width, p = self.prec)?,
                    (false, true) => write!(f, "{:0w$.p$?}", *$v, w = self.width, p = self.prec)?,
                }
            };
            ($to_try:ty $(, $remaining:ty)*) => {
                if let Some(value) = input.downcast_ref::<$to_try>() {
                    trace!("This is a {}!", stringify!($to_try));
                    try_format_tys!(@format value)
                } else  {
                    trace!("This is not a {}", stringify!($to_try));
                    try_format_tys!($($remaining),*)
                }
            };
            () => {
                input.debug(f)?
            }
        }
        try_format_tys!(
            bool, f32, f64, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, String,
            Vec2, Vec3, Vec4, UVec2, UVec3, UVec4, IVec2, IVec3, IVec4
        );
        Ok(())
    }

    fn dyn_clone(&self) -> ShowBox {
        Box::new(*self)
    }
}

pub struct Display<F>(PhantomData<fn(F)>);
impl<F> Clone for Display<F> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}
impl<F: fmt::Display + Reflect + Typed> Show for Display<F> {
    fn format(&self, input: &dyn Reflect, f: &mut fmt::Formatter<'_>) -> Result<(), Error> {
        let downcast = input
            .downcast_ref::<F>()
            .ok_or_else(|| Error::from_reflect::<F>(input))?;
        Ok(downcast.fmt(f)?)
    }

    fn dyn_clone(&self) -> ShowBox {
        Box::new(self.clone())
    }
}

pub struct Convert<I, O, F>(F, PhantomData<fn(I) -> O>);
impl<I, O, F: Clone> Clone for Convert<I, O, F> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}
impl<I, O, F> Convert<I, O, F>
where
    I: Reflect + Typed,
    O: fmt::Display,
    F: Clone + Fn(&I) -> O,
{
    pub(crate) fn new(f: F) -> Box<Self> {
        Box::new(Convert(f, PhantomData))
    }
}
impl<I, O, F> Show for Convert<I, O, F>
where
    I: Reflect + Typed,
    O: fmt::Display + 'static,
    F: Clone + Send + Sync + Fn(&I) -> O + 'static,
{
    fn format(&self, input: &dyn Reflect, f: &mut fmt::Formatter<'_>) -> Result<(), Error> {
        let downcast = input
            .downcast_ref::<I>()
            .ok_or_else(|| Error::from_reflect::<I>(input))?;
        Ok((self.0)(downcast).fmt(f)?)
    }

    fn dyn_clone(&self) -> ShowBox {
        Box::new(self.clone())
    }
}

pub struct ReflectDisplay<'a> {
    show: &'a dyn Show,
    reflect: &'a dyn Reflect,
}
impl fmt::Display for ReflectDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO(err): not sure how to handle errors at this point
        self.show.format(self.reflect, f).map_err(|_| fmt::Error)
    }
}
