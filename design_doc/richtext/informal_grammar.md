# Parsing grammar for rich text 

```
<ident>: [:alpha:_][:alphanum:_]* "identifier respecting rust's identifier rules"
<text∌FOO>: "text that doesn't contain FOO, unless prefixed by backslash `\`
             may be empty"
scope = '{' inner '}' | '(' inner ')' | '[' inner ']'
semi_exposed = <text∌()[]{}>
inner = semi_exposed [scope semi_exposed]*
exposed = <text∌([{}|,>
balanced_text = exposed [scope exposed]*

format_spec = <https://doc.rust-lang.org/stable/std/fmt/index.html#syntax>
format = path ':' format_spec
binding = format | path

path = [:alphanum:_."]+
key = <ident>
open_subsection = <text∌{}>
open_section = <text∌{>
close_section = '{' closed '}'
closed_element = key ':' metadata
closed = format | [closed_element],* ['|' bare_content]?
metadata = '{' format '}' | balanced_text
bare_content = open_subsection [close_section open_subsection]*
rich_text = open_section [close_section open_section]*
```

Rich text is composed of N sections.
Sections are a collection of metadatas plus some content.
Metadatas are values associated with some `key`.
If the section is just an identifer between braces (`{like_this}`),
then it is *dynamic* `Content`.

If the metadata value is between braces (`{like_this}`), then it is *dynamic*.
*Dynamic* metadata can be set and updated at runtime by the user.

A section may end by a `|` followed by text. This represents the text content
of the section.

This text may contain sub-sections, sub-sections may contain other sub-sections.
A subsection is defined similarly to a sections,
but cannot contain the same metadata as the section that contains it, recursively.

`balanced_text` have balanced `[]`, `{}` and `()`, to opt-out of balance
checking for those delimiter, escape them with `\\`.

Since metadata elements are separated by a comma, the `metadata` text must also
escape `,`, otherwise it is considered the end of the value,
unless there is an unclosed open parenthesis or braces.

### Examples

Each line of the following code block represents a valid rich text string.

```
This is some text, it is just a single content section
This one contains a single {dynamic_content} that can be replaced at runtime
{Color:{color}|This is also just some non-dynamic text, commas need not be escaped}
{Content: This may also work\, but commas need to be escaped}
{dynamic_content}
{Color: Blue | This text is blue}
{Color: Blue | {dynamic_blue_content}}
{Color: Blue | This is non-bold text: {Font:bold.ttf|now it is bold, you may also use {RelSize:1.3|{deeply_nested}} sections}, not anymore {b:_|yet again}!}
{Color:Red| Some red text}, some default Color {dynamic_name}. {Color:pink|And pink, why not?}
{Color:rgb(12, 34, 50),Font:bold.ttf|metadata values} can contain commas within parenthesis or square brackets
You can escape \{ curly brackets \}.
{Color: pink| even inside \{ a closed section \}}.
{Color: {relevant_color} | Not only Content can be dynamic, also value of other metadata}
{Content:{ident}} is equivalent to {ident} also {  ident  }.
```

Note that spaces surrounding metadata delimiters are trimmed from the output.

- after `{`
- after `|`
- after `,`

### Counter examples

More importantly, we should provide high quality error messages when things do
not go as planned.

We should also avoid accepting text strings that do something very different
from what the user expects.

All the following strings should result in an error:

```
{some, text, with comma}
```

## Why a custom markup language?

### Why not HTML

- HTML is **HARD** to parse. It is an evolved markup language, it formalizes
  parsing quirks of browser versions from several decades ago.
  This means you are either doing **fake HTML**: breaking users' expectation
  with your quirky parser or pull in a **massive dependency** that solves the
  thousands of edge cases you have to consider when parsing HTML.
- If you opt for the fake HTML route, you force your users to keep in mind at all
  time what differences your markup has with HTML, what they can and cannot do.
  With a clearly distinct markup language, you don't have to adjust expectations.
- HTML is associated with web technologies. Providing HTML implicitly tells our
  users the names of attributes to use, what they are capable of, what they do.
  We will necessarily break those implicit promises in cuicui, so we should not
  make them.
  \
  Take the example of the `<br>` element. Should I do with that?
  Interpret it as a line break? Why not let the user add a line break to their
  input string instead?
  \
  And this isn't to mention stuff like the `style` or `onclick` attributes.

Overall, we are trying to solve a different problem than what HTML is solving.
We just want to display some text with basic styling in bevy,
HTML is not appropriate for that.

### Why cuicui_richtext's markup language

The markup language wasn't designed as a paragon of language design perfection,
in fact, you could say it wasn't designed at all!

But it works and is the perfect fit for the rich text format string.

Unlike HTML, our markup language is not widely known, people aren't already
familiar with it. However, this is a non-issue.

In fact, people _are_ already familiar with it:

- If you've seen JSON before, you understand the concept of
  `a series {of: key, values: within|brackets}`.
- Previous HTML familiarity helps understand distinction
  `{between: attributes|and text}`.
- It also `helps {understand:nested|collections{of:content|that}follow each} other`
- The only trully new and "weird" bit are empty bindings and the `|` to
  start an inline text section.
- If you have a math background, you've already seen `{predicate|declaration}`

I hope cuicui's markup is less complex than HTML.

- Unlike XML, people feel the need to close opened "nodes" by using `}`.
- metadata is consistent, and always declared at the same position.

So I've no doubt people will be able to pick it up quickly.
