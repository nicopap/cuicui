//! Get a string from reflection.
use std::{any::Any, fmt};

use bevy_math::*;

// TODO(feat): make this public, allow trait reflection
struct Formattable(fn(&RuntimeFormat, &dyn Any, &mut fmt::Formatter) -> Option<fmt::Result>);

/// A runtime formatters for rust primitives.
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct RuntimeFormat {
    pub width: u16,
    pub prec: u16,
    pub sign: bool,
    pub debug: bool,
}
pub struct DisplayFormatAny<'a> {
    any: &'a dyn Any,
    format: &'a RuntimeFormat,
}
impl fmt::Display for DisplayFormatAny<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format.format(self.any, f)
    }
}
impl RuntimeFormat {
    fn format_display<T>(&self, v: &T, f: &mut fmt::Formatter) -> fmt::Result
    where
        T: fmt::Display + fmt::Debug,
    {
        macro_rules! write_runtime {
            ($fmt:literal) => {
                write!(f, $fmt, v, w = self.width as usize, p = self.prec as usize)
            };
        }
        match (self.sign, self.debug) {
            (true, false) => write_runtime!("{:+0w$.p$}"),
            (false, false) => write_runtime!("{:0w$.p$}"),
            (true, true) => write_runtime!("{:+0w$.p$?}"),
            (false, true) => write_runtime!("{:0w$.p$?}"),
        }
    }
    pub fn display<'a>(&'a self, any: &'a dyn Any) -> DisplayFormatAny<'a> {
        DisplayFormatAny { any, format: self }
    }
    pub fn format(&self, input: &dyn Any, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn format_any<T: Any + fmt::Display + fmt::Debug>() -> Formattable {
            Formattable(|format, input, f| {
                input
                    .downcast_ref::<T>()
                    .map(|v| format.format_display(v, f))
            })
        }
        macro_rules! all_formats {
            ($( $to_format:ty ),* $(,)?) => {
                    [ $( format_any::<$to_format>() ),* ]
            };
        }
        let formattables = all_formats![
            bool, f32, f64, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, String,
            IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4,
        ];
        for Formattable(format) in formattables.into_iter() {
            if format(self, input, f).is_some() {
                return Ok(());
            }
        }
        Err(fmt::Error)
    }
}
