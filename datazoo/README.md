# The Cuicui Data Zoo

A collection of data structures used in `cuicui_richtext`.
Mostly used for dependency resolution and specialized graph traversal tasks.

Note that this library doesn't work on 16 bits plateforms.
If you need support, consider opening an issue.

You probably need to add [`enumset`] to your dependencies to use this crate.
Due to a rust proc macro limitation, it's impossible to derive `EnumSetType`
without directly depending on `enumset`.

## Limitations

- Data structures are **untested with sizes `> u32::MAX`**
- Effort is made to panic in those situations though, but you never know
- Generally assumes `size_of(usize) >= size_of(u32)`, effort is made to use
  `u32::try_from(usize).unwrap()` though!
- No `#[no_std]` but I don't see why this couldn't be added as a feature

## Data structures

This is a collection of [multimaps], [jagged arrays], [bit sets],
and combination thereof.

See `docrs` documentation for details.

[`enumset`]: https://lib.rs/crates/enumset
[multimaps]: https://en.wikipedia.org/wiki/Multimap
[jagged arrays]: https://en.wikipedia.org/wiki/Jagged_array
[bit sets]: https://en.wikipedia.org/wiki/Bit_array