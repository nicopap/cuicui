# How to handle line breaks in cresus text?

**Problem:** When separating text sections into individual entities, layouting
go bad. No return to new line on `\n`.

Options:

- Create a vertial container where each element is a line of text

Don't see anything better.

## How to manage nested container

So we'd have only a single container. Since the system to query and update items
is customizable, we can introduce our own.

The question should be: "How to detect line ending and split on that?"

Issue: consider a \n in the middle of a section, should we split the section
in two? Then this would screw with the resolver, we shouldn't!

However consider this: if the \n is in the middle of the section.

```sh
this is {|some text} foo bar
↓
["this is " "some text" " foo bar"]
```

```sh
this is {|some text}\nfoo bar
↓
["this is " "some text"]
["foo bar"]
```

```sh
this is {|some\ntext} foo bar
↓
["this is " "some\ntext" " foo bar"] # !!!!!!! (1)
↓
         some
 this is text foo bar
# should be:
this is some
text foo bar
```

(1) causes issue. I don't think I've a solution for this. But already supporting
the rest would be enough.

I could raise an error if we detect a \n neither at the beginning or end of
a section, this would tell the user to split their text in section, so that

What about using `chop` to split on line ending? It can only work right?
**SOLVED**.

## Shape

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃             NODE                  ┃
┃ direction: Vertical               ┃
┃                                   ┃
┃ ┌───────────────────────────────┐ ┃
┃ │           NODE                │ ┃
┃ │ direction: Horizontal         │ ┃
┃ │                               │ ┃
┃ │ ┌────┐  ┌────┐  ┌────┐ ┌─┐┌┐┌┐│ ┃
┃ │ │NODE│  │NODE│  │NODE│ │ ││││││ ┃
┃ │ │Text│  │Text│  │Text│ │ ││││││ ┃
┃ │ └────┘  └────┘  └────┘ └─┘└┘└┘│ ┃
┃ └───────────────────────────────┘ ┃
┃                                   ┃
┃ ┌───────────────────────────────┐ ┃
┃ │           NODE                │ ┃
┃ │ direction: Horizontal         │ ┃
┃ │                               │ ┃
┃ │ ┌────┐  ┌────┐  ┌────┐ ┌─┐┌┐┌┐│ ┃
┃ │ │NODE│  │NODE│  │NODE│ │ ││││││ ┃
┃ │ │Text│  │Text│  │Text│ │ ││││││ ┃
┃ │ └────┘  └────┘  └────┘ └─┘└┘└┘│ ┃
┃ └───────────────────────────────┘ ┃
┃                                   ┃
┃ ┌───────────────────────────────┐ ┃
┃ │                               │ ┃
┃ └───────────────────────────────┘ ┃
┃                                   ┃
┃ ┌───────────────────────────────┐ ┃
┃ └───────────────────────────────┘ ┃
┃ ┌───────────────────────────────┐ ┃
┃ └───────────────────────────────┘ ┃
┃ ┌───────────────────────────────┐ ┃
┃ └───────────────────────────────┘ ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```
