# `impl_modify` atomicisation of nested fields

**Problem:** we can declare a modify function as reading `.foo`, another as
reading `.foo.bar`, another `.foo.baz` etc.

This is a problem for dependency resolution, since we lose the relationship
between `.foo` and its subfields. We need to "specialize" `.foo` into its
atomic accessors.

Note that we can technically have a struct in the shape:

```
Bungle {
  zooba: Zooba
  gee: Gee {
    wooz: Wooz
    zaboo: Zaboo {
      boolo: Boolo
      dodo: Dodo
    }
    wabwab: WabWab
    gomez: Gomez {
      zim: Zim
      zoom: Zoom
    }
  }
  greebo: Greebo
}
```

Accepting the following paths:

```
bungle.zooba
bungle.gee
bungle.gee.wooz
bungle.gee.zaboo
bungle.gee.zaboo.boolo
bungle.gee.zaboo.dodo
bungle.gee.wabwab
bungle.gee.gomez
bungle.gee.gomez.zim
bungle.gee.gomez.zoom
bungle.greebo
```

However, if our `Modify` only accesses `bungle.zooba` and `bungle.gee`, we
absolutely can ignore the rich complexity of the data structure, we only care
about what we access.

The problem occurs when we access both a nested field and the field
containing that field, such as `bungle.gee` and `bungle.gee.zaboo`.

When we run a modify function that changes `bungle.gee`, we need to modify all
functions that depends on `bungle.gee.zaboo`.

## Algorithm

So at one point in our macro, we have the list _A_ of all modify functions, which
themselves have a list of fields they access "accessors".

In this list we need to find the accessors that are non-atomic

An accessor _a0_ is atom of _a1_ when: _a1_ prefix _a0_ and _a1_ ≠ _a0_

An accessor _a1_ is non-atomic when:
∃ _a0_ ∈ _A_, _a0_ atom of _a1_

In which case, we need to collect all the atoms of _ai_ ∈ _A_, and replace
_a1_ by its atoms.

Naive implementation:

* let _M_ be the set of accessors for a modify function
* let _A*_ the set of all **atomic** accessors used by all modify functions
* ∀ _accessor_ in _M_:
  * if _A*_ doesn't contain _accessor_:
    * remove it from _M_
    * then, ∀ _maybe_suffix_ in _A*_:
      * if _maybe_suffix_ atom of _accessor_:
        * insert _maybe_suffix_ in _M_
