# Dynamic format

Problem:

We want to specify an access path to a `Reflect` + a way to translate
it into a `Modify`.

Currently, we only have a "binding", which means we push to format string the
"already digested" `Modify`, format string doesn't care about processing.

But now that we have instructions on what to extract and how to display, this
is a different story.

## The primitive idea

The idea was to "simply" add a modifier called `Format` orsmth. But this doesn't
work.

Because `Modify` doesn't allow reading other `Modify`, and in any case this would
require sorting/associating `Modify`es which seems bad.

## The better idea

So instead of having `Format` be a separate `Modify`, we have it be part of
`Dynamic`.

This has implications for the syntax of the format string.

If we merge `Dynamic` with `Format`, we need to declare them together. Well
_need_ is a bad word. More like "this finally let us".

But we need to ask ourselves: should we change the syntax?
See, currently we have:

```rust
#[derive(Resource, Reflect)]
#[reflect(Resource)]
struct Options {
  audio_volume: f32,
}
```

```
{Content: $Res.Options.audio_volume, Format: {:.0} }
```

With the format and path merged, we get:

```
{Content: $Res.Options.audio_volume:.0 }
```

Fine, but we kinda lose the relationship between the path and the formating,
so what about:

```
{Content: {Res.Options.audio_volume:.0} }
```

Since we have named formatters, we can also do this, also for completness, let's
see what it looks like if we modify the already existing syntax:

```
{Content: $Res.Options.audio_volume:show_audio } // Old syntax
{Content: {Res.Options.audio_volume:show_audio} } // New syntax

{Content: $name_binding }
{Content: {name_binding} }
```

Alternatively, we could do a bash-like `${foobar:baz}`. A major downside is that
now the synax for section is different in content and metadata value position.

This mirrors the rust format syntax, cool. However, it can also be misleading,
as we could think that we can use sections in metadata value position, which
is false; This is a thing entirely different from a section (when in metadata
position), while it's a section in non-metadata position.

It's also a bit extra tricky, because it could be interpreted as a section
with a single metadata field.

I'm not sure what is the best approach. Let's weight the two sides:

**PROS**:

- We can re-use pre-existing syntax, not have to introduce a sigil like `$`
- The grouping between the format element and the binding path is self-evident.
- Can re-use rust knowledge (a bit) to use it
- By re-using, we can take advantage of concept similarity.

**CONS**:

- It does look like rust format strings, but has a few significant differences.
- It does look like a section, but isn't really one
- It might be difficult to parse in a way that allows discriminating with a
  section.

**Idea**: What about _forcing_ a space between `:` and the metadata value in
sections, so that it's always clearly distinct from formats

**Idea 2**: It causes too many parsing woes. I opted to prefix bindings + formats
by `fmt:`, this way the grammar is unambiguous, although weird for users, and
misleading because now we use "format string" both for the whole text and the
special case of bindings that have formatting applied to them.

## Implementation

Making `Dynamic` a struct with a `format` and `access` field doesn't work. As
`Dynamic` **only** accesses bindings by name at runtime (in fact we could
change this to access by an interned ID of sort).

What we really want is a way to say to `RichTextPartial` to register some trackers.

We have things:

- The binding name in `RichText`, it is as before, `modifiers::Dynamic::ByName`.
- A `Tracker` the datastructure that reads from `&World` and updates `modify::Bindings`.

`RichText` builder should return both. Then, the user of `RichText` can add
themselves the tracker to the ECS.
