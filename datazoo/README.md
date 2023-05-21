# The Cuicui Data Zoo

A collection of data structures used in `cuicui_richtext`.
Mostly used for dependency resolution and specialized graph traversal tasks.

You probably need to add [`enumset`] to your dependencies to use this crate.
Due to a rust proc macro limitation, it's impossible to derive `EnumSetType`
without directly depending on `enumset`.

## Data structures

This is a collection of [multimaps], [jagged arrays], [bit sets],
and combination thereof.

See `docrs` documentation for details.

[`enumset`]: https://lib.rs/crates/enumset
[multimaps]: https://en.wikipedia.org/wiki/Multimap
[jagged arrays]: https://en.wikipedia.org/wiki/Jagged_array
[bit sets]: https://en.wikipedia.org/wiki/Bit_array