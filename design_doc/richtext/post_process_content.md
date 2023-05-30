# Splitting Content

## Problem

Splitting by word/char makes it impossible to control a-posteriory the text
vale of split stuff.

It seems impossible to reconciliate.

Splitting requires creating sections based on `Content`.
The `Resolver` depends on the precise section location,
and generally is immutable, I can't remove or add modifiers once the `Resolver`
is generated.

Changing the `Content` of split section would require adding/removing sections.
Which is incompatible with the description I described last paragraph.

## Workaround

It's possible to set the content dynamically _before_ generating the `RichText`.
For example, using the `format!` macro.
However, this is highly annoying, given the `RichText` format syntax.
Need to duplicate all `{}`.

```rust
format!("{{M4 | {{ M2 | {{M1, M3 | Hello}}{{M2, R |{{M1 | This has several}}{{M5 | {user_display} words here }}}}}}{{M6 | also more}}}}")
```

The one user-defined value is `{user_display}`, it's difficult to spot,
and required changing most of the string.

Maybe it could be possible to define a macro with a different format syntax.
We replace `{user_display}` by `$user_display`, no need to escape `{}` anymore.

```rust
richtext_format!("{M4 | { M2 | {M1, M3 | Hello}{M2, R |{M1 | This has several}{M5 | $user_display words here }}}{M6 | also more}}
")
```
