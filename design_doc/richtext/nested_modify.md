# Nested `Modify`

A `Prefab` applies to a series of items. `Modify` are operations on items.

Previous design forbade multiple of the same `Modify` affecting the same item.
This presumes that `Modify` doesn't step on each other's toes. Which is wrong!
There could be a `SetColor` followed by a `LightenColor` `Modify` for example.

Forbidding multiple of the same `Modify` also prevented silly but fun stuff like
nested relative modifications. Think of a text that increases in size progressively:

```
this {RelSize:1.2|text{RelSize:1.2|increases{RelSize:1.2|in size{RelSize:1.2|progressively}}}}
```

Fun is important, we are making video games my dude!


## Implementation

`update` needs an ordering: first run encompassing `Modify`s, then inner `Modify`s.

With the new changed dependency-driven update system it is already the case!

See [./nested_dependency_implementation.md]


## Change detection

This is the 1000 dollar question.
Change detection as currently implemented only supports context changes (root style, bindings).
But nested modifiers require to react to _content_ changes.
Which itself requires keeping track of what a `Modify` changes.

This in itself is costly, yet is not used most of the time
(leaf `Modify`, most of the existing `Modify`s, do not change content read by other `Modify`s).

So, on top of `depends_on`, a `Modify` should declare a `changes` method.
Alternative: `apply` could return a list of things it changed.
This way, we truly minimize change trigger. See [Dynamic Bindings](#dynamic-bindings)

Then, we will use the list of changes + range of `Modify` in `update` to look up
what other `Modify` in `Modifiers` to trigger, and trigger them.

The other aspect to consider is that it seems like I'm building my own ad-hoc ECS.
An item is an entity, a quantum of update is a component, and a `Modify` is a system.

Finally, most `Modify`s do not depend on output of other `Modify`s

### Dependency graph

We need to build a graph of "if this change, then change that".

- `RelSize`
  - DependsOn: _`size`_
  - Changes: _`size`_
- `Color`
  - DependsOn: 
  - Changes: _`color`_
- `Content`
  - DependsOn: 
  - Changes: _`content`_
- `Font`
  - DependsOn: _`font_asset`_
  - Changes: _`font`_
- `Dynamic`
  - DependsOn: _`binding`_
  - Changes: ????
- `ShowSize`
  - DependsOn: _`size`_
  - Changes: _`content`_
- `Uppercase`
  - DependsOn: _`content`_
  - Changes: _`content`_
  
#### Root & leaves

The graph has several roots: each component of the context, including individual
bindings.

The graph's leaves are the components of `item`.

Each `item` has the same _nºc_ components, one component per existing `Changes`.
So the graph has _nºI.c_ leaves: _nºc * nºI_ (_nºI_ = number of items)

#### Edges

Edges are the DependsOn and Changes relationship. 

Each individual root is a distinct DependsOn,
and all DependsOn has a single root.

Changes are the output of `Modify`s.

#### Nodes

- A graph node _M_ is either a root, a `Modify`, or a leaf.
- A node has _0 to N_ inputs _D_, the "DependsOn".
- It has _1 to N_ outputs _C_, the "Changes".
- It has _1 to N_ items _I_ of influence.

#### `Modify` layout

_M1 child of M2_
when (_M1_ is declared after _M2_ in the same item)
or (_M1_'s items of influence is a subset of _M2_
    and not a subset of any other _M3_ child of _M2_).

Exception: The first item of `Content` specified after `|`
_is a parent of_ all other `Modify`s declared in the same item.

> **Note**
> TODO(feat): all `Content` should always run before every other `Modify`s:
> This would allow `Modify`s like `Uppercase` to work with nested `Content`s.

A _child_ `Modify`'s items of influence is always a subset of its parent's.

#### Edge building

_M1 depends(c) on M0_  when:

_M1 child of M0_
and ∄ _M2_,
  _M1 child of M2_
  and _M2 child of M0_
  and _c_ ∈ _M2.C_
and _c_ ∈ _M1.D_
and _c_ ∈ _M0.C_.

_i depends(c) on M_
when ∄ _M0_ such as _M0 child of M_ and _i_ ∈ _M0.I_ and _c_ ∈ _M0.C_.

_i depends(c) on Root.c_
when ∄ _M_ such as _i_ ∈ _M.I_ and _c_ ∈ _M.C_.

### Heuristics

- k depends(c) on Root(c) for most k (item, `Modify`).
- i depends(Content) on a leaf `Modify` always
- Content is always leaf.
- dependencies of different c often do not interleave (ie: distinct trees)
- `Modify`s that do not DependOn a Root (recursively) can be culled, as long
  as we don't touch the final item component they Change.

### What proofs do I need?

- It is fine to cull static modifiers.

#### Mask system works

_M_ does nothing when:

∀ _i_ ∈ _M.I_, ∀ _c_ ∈ _M.C_,
  ∃ _M1_,
    _M1 child of M_
    and _i_ ∈ _M1.I_
    and _c_ ∈ _M1.C_
and ∀ _c_ ∈ _M.C_,
  ∀ _M1 child of M_,
    _c_ ∉ _M1.D_.

_M_ is static when:

∀ _d_ ∈ _M.D_,
  ∃ _M1 is static_,
    _M depends(c) on M1_
or _M does nothing_

This means that whatever changes in the future, the component will always have
the same value, we can just apply them once and forget their value.

We cull "static" modifiers from the `Resolver`. 
That means that if we ever modify that item component,
we forever lose its initial value, entering a wrong unrecoverable state.

Static modifiers can be nested in a non-static modifier.
Since we store modifier application range as a range rather than a list of
indices, we need to be careful to not overwrite the value of static modifiers
nested within that range.

This is why we have a mask. The mask keeps track of item indices of culled modifiers.
We check against it when applying a modifier, skipping the indices we know we
shouldn't apply.

_M0 masks(c)_
when _c_ ∈ _M0.C_ and _M0.D_ = ∅

When we update _M1_, we update all component _c_ ∈ _M1.C_ of _M1.I_

_Upd(M1)_
implies ∀ _i_ ∈ _M1.I_, _c_ ∈ _M1.C_: _changed(i.c)_

The tricky bit is the colored relations. Can I abstract it away?
Saying X depends on Y, or update X always follow update Y allows using
compact bitfields.

- Assuming distinct trees (`depends_on: Option<Change>`), can we assume
  dependencies to work globally?
  → Trivially: **NO**.

### Dynamic bindings

Issue: `modifiers::Dynamic` can't know at compile-time its dependencies,
and what it changes, it's behavior depends on what the user sets it to.

Idea: store a `Vec<DependsOn>` or `PhantomData<T>` in `Dynamic`.
This resolves the issue of dynamic binding type specification as well.

### Open problems

#### Where to store my `ModifyBox`es?

The final data structure would be like

```rust
/// Index in `modifies`.
struct ModifyIndex(u32);

struct RichText {
  /// All `ModifyBox` that can be applied.
  modifies: Box<[ModifyBox]>,
  direct_dependencies: Box<VarMatrix<ModifyIndex, Change::BIT_WIDTH>>,
  modify_dependencies: BitMultiMap<ModifyIndex, ModifyIndex>,
  dynamic: Vec<(BindingId, ModifyIndex)>,
}
```

However, why the level of indirection and storing `ModifyIndex` in `VarMatrix`
over, say, `ModifyBox` already?

This would prevent the same `Modify` to depend on more than one `Change`.
A use case that is wishable long term, but not immediately required.

→ This would also prevent `Modify`s that purely depend on other `Modify`s,
so it isn't possible.

#### Range dependency mask

We store the range of items a `Modify` influences. But within that range,
some item components are masked by other child `Modify`s, the question on
how to solve this is open.

Supposedly,
we store a bitset containing the mask of items with which depends on
`Modify`s that themselves depend on nothing.
This way, we never update them.
Updating other items is actually necessary, as we run the depending `Modify`
with the new values.

#### `Binding`s representation

We handily removed bindings from our considerations, but they are important.

I'm thinking of a `Vec<(BindingId, (u16, u16))>` sorted vec where (u16, u16)
is position in `VarMatrix` of relrevant `Modify` → Does not work
because may have several `Modify` per binding.

We could make a `VarMatrix` where the key is association `(BindingId, starts_at)`.

## Binding to Modify dependencies

Problem:

- Static culling spuriously deletes `Modify` that only depends on bindings.

Solution: Do not cull `Modify` that only depends on bindings.
