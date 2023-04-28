# Parsing grammar for rich text 

```
<ident>: "identifier respecting rust's identifier rules"
<text∌FOO>: "text that doesn't contain FOO, unless prefixed by backslash `\`"
<balanced_text∌FOO>: "Same as <text> but, has balanced brackets and can
                      contain unescaped FOO within brackets"
closed_element = key ':' metadata
key = 'font' | 'content' | 'size' | 'color'
sub_section = '{' closed '}' | <text∌{}>
bare_content = (sub_section)+
metadata = '$' <ident> | <balanced_text∌,}|>
closed = <ident> | (closed_element),* ('|' bare_content)?
section = '{' closed '}' | <text∌{>
rich_text = (section)*
```

Rich text is composed of N sections.
Sections are a collection of metadatas.
Metadatas are values associated with some `key`.
With no specified `key`, if the section is just an identifer between braces
(`{like_this}`) then it is *dynamic* `content`.
If the metadata text is a `$` followed by an identifer, then it is *dynamic*.
*Dynamic* metadata can be set and updated at runtime by the user.

A section may end by a `|` followed by text. This represents the text content
of the section.

This text may contain sub-sections, sub-sections may contain other sub-sections.
A subsection is defined similarly to a sections,
but cannot contain the same metadata as the section that contains it.

`balanced_text` have balanced `[]`, `{}` and `()`, to opt-out of balance
checking for those delimiter, escape them with `\\`.

Since metadata elements are separated by a comma, the `metadata` text must also
escape `,`, otherwise it is considered the end of the value,
unless there is an unclosed open parenthesis.

### Examples

Each line of the following code block represents a valid rich text string.

```
This is some text, it is just a single content section
This one contains a single {dynamic_content} that can be replaced at runtime
{color:$|This is also just some non-dynamic text, commas need not be escaped}
{content: This may also work\, but commas need to be escaped}
{dynamic_content}
{}
An empty {} is equivalent to {name}, but referred by typeid instead of name
{color: Blue | This text is blue}
{color: Blue | {dynamic_blue_content}}
{color: Blue | This is non-bold text: {font:bold.ttf|now it is bold, you may also use {size:1.3|{deeply_nested}} sections}, not anymore {b:_|yet again}!}
{color:Red| Some red text}, some default color {dynamic_name}. {color:pink|And pink, why not?}
{color:rgb(12, 34, 50),font:bold.ttf|metadata values} can contain commas within parenthesis or square brackets
You can escape \{ curly brackets \}.
{color: pink| even inside \{ a closed section \}}.
{color: $relevant_color | Not only content can be dynamic, also value of other metadata}
{color: $ |If the identifier of a dynamic metadata value is elided, then the typeid of the rust type is used}
can also use a single elided content if you want: {content:$}
{content:$ident} is equivalent to {ident} also {  ident  }.
```

Note that spaces surrounding metadata delimiters are trimmed from the output.

- right side of `{`
- left side of `}`
- both sides of `|`
- both sides of `,`
- right side of `:`

### Counter examples

More importantly, we should provide high quality error messages when things do
not go as planned.

We should also avoid accepting text strings that do something very different
from what the user expects.

All the following strings should result in an error:

```
{some, text, with comma}
{Intelligence: very bad}
```
