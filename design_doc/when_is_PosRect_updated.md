## Optimizing layouting algorithm

Walking through the whole layout tree each frame is not ideal.
Likely easy optimization because very streightforward.

What are the input of a given entity's PosRect?

PosRect is:

- Size
  - Container
    - cross is always depends on max size of children
    - axis for Compact depends on total size of children
    - axis for Stretch depends on parent axis (equals)
  - Spacer
    - axis depends on size of parent
    - Also depends on field parent_ratio value
  - Static
    - only depends on self
- Pos
  - is set by parent always

For containers (Stretch):
If self not changed, and no children changed, and parent axis not changed,
then no need to update.

For containers (Compact):
Independent from parent so:
If self not changed, and no children changed,
then no need to update.

For spacers:
if parent axis not changed, and self not changed,
then no need to update.

For static:
if self not changed, then no need to update.

