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