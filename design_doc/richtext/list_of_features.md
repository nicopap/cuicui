# List of features

This file should help design tests and examples that enable quickly debugging
what's going on.

### `datazoo`

See modules and data structure methods

- `bitmatrix`
- `bitmultimap`
- `bitset`
- `enum_bitmatrix`
- `enum_multimap`
- `jagged_bitset`
- `jagged_const_row_array`
- `jagged_vec`
- `sorted`

### `fab_derive`

- arguments passed to `impl_modify` directly:
  - `cuicui_fab_path = alternate::path`: specify which path to use for the
    `cuicui_fab` crate by default, it is `::cuicui_fab`
  - `enumset_crate = identifier`: specify which path to use for the `enumset`
    crate by default, it is `enumset`
  - `no_derive(Debug | Clone)`: Do not automatically implement given trait for Modifier.
  - `visibility = [pub(crate)]`: specify the visibility for the generated enums.
    by default, it is `pub`
- Generate `<target>Field` enum with all fields
- Generate `<target>` enum
  - `#[doc(hidden)]` taken from modify function privacy
  - doc strings taken from same
  - Additional paragraph telling which fields it accesses
- Generate `impl <target>` block with constructors 
  - Privacy taken from modify function
  - documentation from same
- Generate `impl <target>` block with modify function depends/changes
  - Do not generate empty ones
- Generate `changes` and `depends` associated func of `Modify` based on declared
  access pattern.
- modify attributes:
  - `context([ident =] .path.in.context)`: The context declared in `type Context = Foo;`
  - `write(.path.in.item)`: path in item to write return value
  - `write_mut([ident =] .path.in.item)`: write-only path in item to pass as `&mut ident`
  - `read([ident =] .path.in.item)`: read-only path in item to pass as `&ident`
  - `read_write([ident =] .path.in.item)`: read/write path in item to pass as `&mut ident`
  - `dynamic_read_write(read_ident, write_ident [, ident])`: pass `&mut item` and read
    those fields for checking which paths in item are read from and writen to.
    The thirs optional parameter is which function argument to pass it to
    (by default it is `item`)
  - modify attribute path components:
    - `[0]` literal int indexing
    - `["test"]` literal string indexing
    - `.field` field access
    - `.0` tuple field access
  - modify attribute path format:
    - `ident = .full.path` call the modify function with value at `.full.path`
      for parameter named `ident`
    - `.full.path` call the modify function with value at `.full.path`
      for parameter named `path` (last identifier)
    - `just_ident` call the modify function with the whole `Item` or `Context`

### `fab`

- `MinResolver` & `Resolver`
  - Generate a correct `Items` based on initial `MakeModify` list, based on its
    static values
  - Keep track of `MakeModify::binding` of exactly one item range, and be able
    to update them
  - Do not store static modifiers, since they won't ever change in the future.
- `Resolver`-specific
  - Apply +1 range bindings
  - Do not overwrite static modifiers that are within range of updated ones
  - This includes bindings
  - Do re-run modifiers that depends on updated items
  - This, but also deeply nested
- `binding::World` & `binding::Local`
  - Keep a list of bindings -> Modify
  - Keep track of which bindings were updated since last `reset_change`
  - `Entry` API to update Modify
  - An `#[repr(u32)] Id` to use over `String` for the API, this reduces overhead.
- `binding::World`-specific
  - An interner that returns the `Id` through `get_or_add`
- `binding::Local`-specific
  - Keep track of the `&str` -> `Id` already encountered to avoid costly interning
- `binding::View`
  Overlay a `Local` over `World` so that `World` bindings are shadowed locally

### `fab_parse`

- `rt_fmt` Runtime formatting, supports printing using dynamic strings, see
  [rust fmt module]
  - width
  - fill
  - alignment
  - sign
  - alternate
  - binary/hexadecimal/octal
  - precision
  - user-provided custom formatters
  - fallback `Reflect` usage
- `parse` parsing the format string [see grammar spec]
  - sequence of sections
  - nested sections
  - modifiers
  - special formatting
  - binding
  - modifiers binding
  - balanced text on modifier values
- `post_processing` special transformations on parse tree
  - `alias`
    - as text: convert a modifier into other modifiers, but textual still
    - as modifier: convert a modifier into a list of actual modifier values
  - `chop`: split a modifier and generate one subsection per word or character
    with a modifier provided by a function evaluated once per generated
    subsection
    - By curve
    - With accumulator
    - with arbitrary function
    - Handle nested subsections properly

### `bevy_fab`

- `tracker`s
  - system reading hooks and updating associated bindings with values of
    hook source
  - **Unimplemented**: avoid updating bindings when the source didn't change,
    using bevy's ECS change detection system
  - **Unimplemented**: avoid updating bindings when the source didn't change,
    by caching the old value and comparing to the new one.
  - binding source queries
    - Efficient caching of functions used to read from world the `Reflect`
      components/resources & access the reflected `GetPath`.
    - `Res`
    - `One`
    - `Name`
    - `Marked`
  - `TrackerBundle` a bundle to automatically add a `Hook` reading a component
    it wraps.
    - `TrackerBundle::debug`: prints the `fmt::Debug` of component,
      can be disabled with feature flag
    - `TrackerBundle::content`: prints the `fmt::Display` of component.
    - `TrackerBundle::modifier`: sets binding to `Modifier::from<&Comp>` of component.
  - Parse into a tracker the `Hook` returned by `fab_parse`
  - Format either using display or directly into `Modify`
    - Avoids allocation when possible
    - use `rt_fmt` when it is relevant.
    - User-provided custom formatters here as well.
- Manage in the ECS the `binding::World`, `Resolver` and `binding::Local`
- `ParseFormatString`: Allow user to spawn and create `Resolver`s indirectly
  through a "make" component, that is then read in an exclusive system that
  updates bindings and creates hooks based on the format string

### `richtext`

- provide an API completely devoid of the complexity of all the fab crates


[rust fmt module]: https://doc.rust-lang.org/stable/std/fmt/index.html
[see grammar spec]: https://github.com/nicopap/cuicui/blob/main/design_doc/richtext/informal_grammar.md