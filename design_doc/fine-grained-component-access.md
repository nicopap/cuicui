# Fine-grained component access

I've had this crazy idea just earlier. **fine-grained component access**

So suppose we have a generalized tool to go from a `T: Reflect + Component` to
`R` a field of `T` (some call it lens).

For example, we could have a `WorldQuery` that takes `T` and a path,
say `Path![T, .field.foo]`, and can only a access the field in question.

This adds some interesting properties, we could imagine integrating
this with the scheduler, it would enable parallelizing systems that access
mutably the `translation` and `rotation` fields of `Transform`.
We could also imagine an extension to change detection
that manages path projections.