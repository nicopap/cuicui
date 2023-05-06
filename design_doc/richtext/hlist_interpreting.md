# Of the use of Hlists in `RichText`

A hlist aka "heterogenous list" is a fancy tuple of multiple different types
that all implement the same trait.

Instead of creating a `HashMap<&'static str, (TypeId, MakeModifyBox)>`, we could
pass as type parameter a hlist.

Now, in `interpret`, we could iterate over the `hlist` and use the appropriate
`Modify::parse` function.

Furthermore, instead of storing a `GoldMap<TypeId, ModifyBox>` per section, we
could store a `Box<dyn ModifyList>`. This avoids pointer chasing, since instead
of storing a pointer to our `Modify`, we store the `Modify` itself as a field
of an anonymous `struct` (the hlist).

That seems insane and highly unlikely! Why? Well first off we need the
_exhaustive_ list of types when calling `interpret` on the _list of sections_.

```rust
fn interpret<H>(sections: Vec<parse::Section>) -> RichText {
  //...
}
```

Then, for each individual section, you need to produce a distinct list of types
based on the list itself (😰). Our one advantage is that the produced list of
type is erased. But it should really be distinct, as each section will have
a different number of types.

The list of types to return depends on a runtime value, so I'm actually not quite
sure it's possible.

Where `ModifyList` is a hlist of `Modify`, particularity is that the 