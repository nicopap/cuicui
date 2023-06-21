mod atom;
mod parse;
mod split_borrow;

/// A component of a [`Path`] from a source value to a target value.
///
/// Each `Access` goes from a "containing" type (such as a `struct`, a tuple or an array)
/// to a "contained" type. Recursively, until the final access is reached.
#[derive(PartialEq, Debug, Clone, Copy)]
enum Access<'a> {
    /// <https://doc.rust-lang.org/reference/expressions/field-expr.html>
    Field(&'a str),
    /// <https://doc.rust-lang.org/reference/expressions/tuple-expr.html#tuple-indexing-expressions>
    TupleIndex(usize),
    /// <https://doc.rust-lang.org/reference/expressions/array-expr.html#array-and-slice-indexing-expressions>
    ArrayIndex(usize),
}
struct Path<'a>(Box<[Access<'a>]>);
