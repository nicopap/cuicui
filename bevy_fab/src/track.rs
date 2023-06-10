//! Tracker structs to easily insert into ECS components you want to read
//! into modifiers.

mod read;
mod write;

pub use read::{GetError, ParseError, Read};
pub(crate) use write::UserWrites;
pub use write::{Error as WriteError, UserWrite, Write};
