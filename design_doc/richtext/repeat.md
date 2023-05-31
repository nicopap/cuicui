# Repeating modifiers

We want like real fancy text transforms. The stapple is Paper Mario's dialog box,
we aim to give users the ability to recreate it.

Paper Mario's dialog box included moving letters, words, changing colors.
But also "typewritter effect", animations that play directly at the beginning
of the letter appearing, input advancing text typewritting; letters moving
one after another (for example, as a wave)

We already have the capacity to reproduce this!

But let's consider all design possibilities.
By first considering each use case and how users could accomplish those tasks.

## Nesting dependent modifier

Consider `HueShift`, that shifts color hue by a set amount out of the 360º of
hue.

```
{Color:{base_hue}|R{HueShift:50|A{HueShift:50|I{HueShift:50|N{HueShift:50|B{HueShift:50|O{HueShift:50|W}}}}}}}
// ^^ defines nested section for each letter, where the inner section shift the hue
// of the encompassng section by 50º
[R[A[I[N[B[O[W]]]]]]]
```

That format string reads a base color from `base_hue`, then shifts progressively
by 50º the color for each subsequent letter.

First the `base_hue` is applied to all the letters `[RAINBOW]`.

Then, the first HueShift is applied to `[AINBOW]`

Second to `[INBOW]` -> `[NBOW]` -> `[BOW]` -> `[OW]` -> `[W]`

The result is that each following letter is shifted an additional 50º hue.
If we animate `base_hue`, we get a really nice looking shifting rainbow effect.

The syntax for this is not very user-friendly. We in fact barely can
guess that this spells out "RAINBOW". So let's define a special modifier
(similar to Content)

```
{Color:{base_hue}, ShiftHue:50, LetterRepeat:ShiftHue|RAINBOW}
```

`LetterRepeat` (consider that we could also have a `WordRepeat`, splitting on
words rather than letters) accepts a single or a list delimited by `[]`
of modifiers to apply repetitively to each letter in the content value.

This would:

- split each letter into its own section, each letter is part of a different
  level of nesting
- Give a copy of the `Modifier`s specified as arguments to each subsection

### Problems

- Time complexity: This requires applying **½ · n²** times the `HueShift` modifier.
- Preset subsection splitting: This prevents using subsections in the content string.
- Verbosity: This is still fairly verbose, although not as much as before

### Alternative: flat shift

We could consider letting user define modifier names. And instead of a deeply
nested set of sections, each letter get its own section. Now we only have
to run `HueShift` once per letter.

```rust
use Repeat::ByLetter;
make_rich_text("{Rainbow:0.0|RAINBOW}")
  .repeat_modifier(ByLetter, "Rainbow", Modifier::hue_shift, |i| i + 50.0)

fn repeat_modifier<I, M, S>(
  mut self,
  repeat: Repeat,
  alias: &str,
  mk_modifier: M,
  shift_value: S,
) -> Self
where I: FromStr + Clone,
      M: Fn(I) -> Modifier,
      S: Fn(I) -> I,
```

Now, We create a new `ShiftHue` with provided shift per letter

```
{Rainbow:0.0, Color:{base_hue}|RAINBOW}
[[R][A][I][N][B][O][W]]
```

This actualy solve all our problems:

- Time complexity: The shift is done once per letter, while the `Color` is set
  for all of them once
- Subsection splitting: Since it is split on the character level, we can handle
  encompassing subsections; This also works with word-level splits, since subsection
  do split words
- Verbosity: We now use a single alias, not several modifiers + a modifier list, hurray.

However, this would require defining it in rust code, not possible to create
ad-hoc repeat modifiers just within the format string.

### Aliases

Maybe user want to create their own composition of modifiers.

The current way of doing this would be to define a `TextModifier` and load it
as a dynamic modifier. This require bunch of code, is error prone and not
very efficient.. Composition works by using the primitive modifiers in the
custom modifier.

What about letting user provide a list of "aliases"? It would look like:

```rust
make_rich_text("{Rainbow:(),|RAINBOW}")
  .alias("Rainbow", &[
    Modifier::letter_repeat("HueShift"),
    Modifier::hue_shift(50.0),
  ])
```

`LetterRepeat` is not really a modifier, since it requires splitting the underlying
string into more items, this can only be done by the format string parser.

But it should be possible to pass the alias list to the parser so that they can
use it to transform the parsed value.

## Cubic curve support

Bevy has a `CubicCurve` struct. It seems fairly easy to handle with `repeat_modifer`

## Typewriter effect

## Input advances dialog

## Translate to other languages

Since we parse a string at runtime, it is possible to load different strings
depending on a runtime or static setting.

But the DSL for rich text should be approachable enough for non-technical people
to use effectively.

## Implementation

Splitting either happens during parsing or as a transformation after parsing.

### At parse time

Advantages are:

- Do not need to re-visit the parsed tree, less overhead
- Easier to keep track of section size and position

Problems are:

- Current impl is only a bunch of `&str` there is no semantics to parsed value,
  would need to introduce storing `Modify` to the parsing
- If we opt for a hlist-based `Split` declaration, we would need to pass the
  `S: Split` is type parameter accross all parsing functions, and might result
  in very difficult-to-read error messages.
- Mixing semantics with syntax parsing probably won't end well.

Implementation:

I would change the way `Sections` extends its internal list. I would read the
`Content` modifier and split the sections in multiple sections before accumulating
them.

Actually change `flatten_section` and `Section::free`.

would need to add a `s: Split` argument to those functions? Add a field to
`Section` to account for that. Then the returned `Vec` of those functions would
be extended based on the repeat mode. (but also the modifier lengths shall be
extended as well.

### Post-parsing

Each `Section` has `Modifier`s that apply to the current `content`, but also
all the next *n* `Section`s where *n* is `Modifier::subsection_count`.

```
{M4|{ M2|{M1, M3|Hello}{M2, R|{M1 |This has several}{M5|words here}}}{M6|also more}}
Four sections:   -----             ----------------     ----------       ---------

R: (Repeat::ByWord, fn(_) -> M8)

  Hello  │      This has several      │    words here    │  also more
M2───────┼────────────────────────────┼──────────────    │M6─────────
M1─────  │  M2────────────────────────┼──────────────    │
M3─────  │  R ────────────────────────┼──────────────    │
         │  M1————————————————————    │  M5──────────    │
M4───────┼────────────────────────────┼──────────────────┼───────────

  Hello  ┃  This  │  has  │  several  ┃  words  │  here  ┃  also more
M2───────╂────────┼───────┼───────────╂─────────┼──────  ┃M6─────────
M1─────  ┃M2──────┼───────┼───────────╂─────────┼──────  ┃
M3─────  ┃M8────  │M8───  │M8───────  ┃M8─────  │M8────  ┃
         ┃M1──────┼───────┼─────────  ┃M5───────┼──────  ┃
M4───────╂────────┼───────┼───────────╂─────────┼────────╂───────────

=====================================================================
=====================================================================

┌───────┬──────────────────────┬────────────────┬─────────┐
│ sec 1 │         sec 2        │     sec 3      │  sec 4  │
│ Hello │   This has several   │   words here   │also more│
├───────┼──────────────────────┼────────────────┼─────────┤
│mod│len│        mod│len       │     mod│len    │ mod│len │
│───┼───│        ───┼───       │     ───┼───    │ ───┼─── │
│M4 │ 4 │        M2 │ 2        │     M5 │ 1     │ M6 │ 1  │
│M2 │ 3 │        R  │ 2        ├────────────────┴─────────┘
│M1 │ 1 │        M1 │ 1        │
│M3 │ 1 ├──────────────────────┘
└───────┘

┌───────┬───────┬───────┬───────┬───────┬───────┬─────────┐
│ sec 1 │ sec 2 │ sec 3 │ sec 4 │ sec 5 │ sec 6 │  sec 7  │
│ Hello │ This  │  has  │several│ words │  here │also more│
├───────┼───────┼───────┼───────┼───────┼───────┼─────────┤
│mod│len│mod│len│mod│len│mod│len│mod│len│mod│len│ mod│len │
│───┼───│───┼───│───┼───│───┼───│───┼───│───┼───│ ───┼─── │
│M4 │ 7 │M2 │ 5 │M8 │ 1 │M8 │ 1 │M8 │ 1 │M8 │ 1 │ M6 │ 1  │
│M2 │ 6 │M8 │ 1 ├───────┴───────┤M5 │ 2 ├───────┴─────────┘
│M1 │ 1 │M1 │ 3 │               └───────┘
│M3 │ 1 ├───────┘
└───────┘
```

What happens when we have multiple active `Repeat`?
⇒ Let's leave this for the future.

- For each Possible `repeat`:
  - Iterate through all sections, if `section` has given `repeat`:
    - Remove `repeat` from `section`
    - Count sum of words in next `section.repeat.subsection_count` (including this one)
    - For `current_section` in next `section.repeat.subsection_count` (including this one):
      - Split content in *n* contents in `content_list`
      - Change `current_section`'s content to `content_list.head`
      - For all `modify` in `current_section`:
        - Increment `modify.subsection_count` by *n - 1*
      - For all `section` precedeeing `current_section`:
        - For all `modify` in `section`, if `subsection_count` contains `current_section`:
          - increment `modify.subsection_count` by *n - 1*
      - Add the `mk_modify` with `subsection_count = 1` to `current_section`
      - For all `content` in `content_list.tail`:
        - Insert between `current_section` and the next a new section
          with only modifiers `mk_modify` with `subsection_count = 1` and
          `content`
