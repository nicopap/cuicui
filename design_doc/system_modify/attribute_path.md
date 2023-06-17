# How to specify paths at the type level?

**Problem:** 

I have an `Attribute` trait. The idea is to pass to an `App` extension method
the path to read the `GetPath` value.

I want the modify system to look as follow:

```rust
fn my_modfiy(state: State<Foobar>, translation: Write![Transform; ".translation"]) {
  // do thingies!
}

// ...

app
  .add_modify(my_modify)
```

But it's not possible. In stable rust, it's simply impossible to express a `&str`
in the type system (which would be necessary to convert a simple function
into a `Modifier`).

I considered using a bunch of `u8` generic parameters, but the string has vairable
width, so not possible.

What we need is to instantiate a `&str` even if `const` or whatever. So the
end result would look like this maybe?

```rust
#[modify]
fn my_modfiy(state: State<Foobar>, translation: Write![Transform; ".translation"]) {
  // do thingies!
}
const my_modify = Modifier {
  arg_paths: (".translation",),
  function: |state: State<Foobar>, translation: Write<Transform>| {
    // do thingies!
  }
}

// ...

app
  .add_modify(
    (".translation", ".size.width"),
    |translation: Write<Transform>, width: Read<Style>| {
      // do other thingiess.
    },
  )
```

Due to a limitation in rust's type system. I can't store the path in
the type and read it from an associated method.

This means I need to find a workaround.

