# Nested `Modify`

Previous design forbade multiple of the same `Modify` affecting the same section.
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
A section is an entity, a quantum of update is a component, and a `Modify` is a system.

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

The graph's leaves are the components of `Section`.

Each `Section` has the same _NC_ components, one component per existing `Changes`.
So the graph has _NSC_ leaves: _NC * NS_ (_NS_ = number of sections)

#### Edges

Edges are the DependsOn and Changes relationship. 

Changes is a subset of DependsOn. Each individual root is a distinct DependsOn,
and all DependsOn has a single root.

Changes are the output of `Modify`s.

#### Nodes

- A graph node _M_ is either a root, a `Modify`, or a leaf.
- A node has _0 to N_ inputs _D_, the "DependsOn".
- It has _1 to N_ outputs _C_, the "Changes".
- It has _1 to N_ sections _S_ of influence.

#### `Modify` layout

_M1 child of M2_
when (_M1_ is declared after _M2_ in the same section)
or (_M1_ is a subsection of _M2_).

Exception: The first section of `Content` specified after `|`
_is a parent of_ all other `Modify`s declared in the same section.

> **Note**
> TODO(feat): all `Content` should always run before every other `Modify`s:
> This would allow `Modify`s like `Uppercase` to work with nested `Content`s.

A _child_ `Modify`'s sections of influence is always a subset of its parent's.

#### Edge building

_M1 depends(c) on M0_ 
when _M1 child of M0_ and _c_ ∈ _D(M1)_ and _c_ ∈ _C(M0)_.

_s depends(c) on M_
when ∄ _M0_ such as _M0 child of M_ and _s_ ∈ _S(M0)_ and _c_ ∈ _C(M0)_.

_s depends(c) on Root(c)_
when ∄ _M_ such as _s_ ∈ _S(M)_ and _c_ ∈ _C(M)_.

### Heuristics

- k depends(c) on Root(c) for most k (section, `Modify`).
- s depends(Content) on a leaf `Modify` always
- Content is always leaf.
- dependencies of different c often do not interleave (ie: distinct trees)

### What proofs do I need?

I don't know if "ordered" trigger is enough to:

- Apply all `Modify`s affected by a change
- Apply no other `Modify`s than the ones affected by a change

The difficulty is compounded when considering that dependency types are
intermixed in the algorithm.

### Dynamic bindings

Issue: `modifiers::Dynamic` can't know at compile-time its dependencies,
and what it changes, it's behavior depends on what the user sets it to.

Idea: store a `Vec<DependsOn>` or `PhantomData<T>` in `Dynamic`.
This resolves the issue of dynamic binding type specification as well.