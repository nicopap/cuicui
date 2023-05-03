//! Get a string from reflection.
use std::{fmt, marker::PhantomData};

use bevy::{
    reflect::{Reflect, Typed},
    utils::{get_short_name, HashMap},
};
use thiserror::Error;

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

pub trait RichShow {
    fn format(&self, input: &dyn Reflect, f: &mut fmt::Formatter<'_>) -> Result<(), Error>;
}

pub struct Display<F>(PhantomData<F>);
impl<F: fmt::Display + Reflect + Typed> RichShow for Display<F> {
    fn format(&self, input: &dyn Reflect, f: &mut fmt::Formatter<'_>) -> Result<(), Error> {
        let downcast = input
            .downcast_ref::<F>()
            .ok_or_else(|| Error::from_reflect::<F>(input))?;
        Ok(downcast.fmt(f)?)
    }
}

pub struct Convert<I, O, F>(F, PhantomData<fn(I) -> O>);
impl<I, O, F> Convert<I, O, F> {
    fn new(f: F) -> Box<Self> {
        Box::new(Convert(f, PhantomData))
    }
}
impl<I, O, F> RichShow for Convert<I, O, F>
where
    I: Reflect + Typed,
    O: fmt::Display,
    F: Fn(&I) -> O,
{
    fn format(&self, input: &dyn Reflect, f: &mut fmt::Formatter<'_>) -> Result<(), Error> {
        let downcast = input
            .downcast_ref::<I>()
            .ok_or_else(|| Error::from_reflect::<I>(input))?;
        Ok((self.0)(downcast).fmt(f)?)
    }
}

type RichShowBox = Box<dyn RichShow + Send + Sync + 'static>;

pub struct RichTextBuilder {
    text: &'static str,
    formatters: HashMap<&'static str, RichShowBox>,
}
impl RichTextBuilder {
    pub fn with<I, O, F>(mut self, name: &'static str, convert: F) -> Self
    where
        I: Reflect + Typed,
        O: fmt::Display + 'static, // TODO(bug): shouldn't need this + 'static
        F: Fn(&I) -> O + Send + Sync + 'static,
    {
        self.formatters.insert(name, Convert::new(convert));
        self
    }
}
