#![doc = include_str!("../README.md")]
#![warn(clippy::nursery, clippy::pedantic, missing_docs)]
#![allow(clippy::use_self)]

mod custom_ref;
pub mod predefined;
pub mod queries;
mod queryable;

pub use custom_ref::Ref;
pub use queryable::{ReflectQueryable, ReflectQueryableFns};
