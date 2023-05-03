# Binding Contexts

A `RichText` has multiple sources for context:

- `RichTextData`, a component neighbor to `RichText` that can be queried, it
  allows:
  - *by-name* bindings: you give the name in the format string, and use the
    `set("name", …)` method on `RichTextData`.
  - *by-type* bindings: you elide the name (with `$` or `{}`) and you use the
    `set_typed` method on `RichTextData`. Since `RichText` associates binding
    with a unique `Modify`, it can read the value for the associated type.
- `WorldContext` a resource.
  - By manipulating it directly with one of its mehtods.
  - Through the `track` API, which associates a component or resource to a
    binding name


## Future work

### Pull vs push API

With the `track` API, we force the user to declare at to different locations
what they want to read:

- In the format string, with the `{binding_name}` or `$binding_name`
- When adding the component/resource in question, using `track!` or
  `insert_tracked_resource`.

Typically, for resource, we refer in the format string **by the name of the
resource type**, which is why we require the resource to derive `Reflect`.

For example, for a resource `struct Foobar(u32)` we will need to declare the
binding in the format string as `our foobar: {Foobar}`.

So we already know the name of the type we want to extract. Isn't that enough?
Shouldn't cuicui_richtext be able to read that and use it to _pull_ the value
of the resource, rather than waiting that we _push_ it into it?

It probably can, with `TypeRegistry::get_with_name`.