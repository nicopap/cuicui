# Layout containers

Due to explicit limitations, we need better containers.
Specifically, containers with set sizes, and maybe sizes expressed
as percentage of parent container.

Why?

1. Explicit limitations (ambiguous situations such as two horizontal stretch
   within a horizontal stretch) should be detected and result in warning.
2. layout update could have much more fine-grained work skipping heuristic
3. it is easier and more expected to specify the container with its constraints

We could simply define `Spacer` and `Static` as potential containers themselves,
but this adds additional constraints: for example, we now need to check they
have only a single child, propagate the constraints downwards coherently etc.

Assume `Static` and `Spacer` are leaf nodes only. They are for terminal
nodes in the UI such as text or image widgets.

Remember dependencies from `when_is_PosRect_updated.md`:

- Size
  - Container
    - cross always depends on max size of children
    - axis for Compact depends on total size of children
    - axis for Stretch depends on parent axis (equals)
  - Spacer
    - axis depends on size of parent
    - Also depends on field parent_ratio value
  - Static
    - only depends on self
- Pos
  - is set by parent always

When do I know the parent sizes?

- Has a set size
- The axis in parent Stretch container direction
  => Is it true? => Only if you know parent's parent container axis size
  => how to fix? => Propagate uncertainty downard
