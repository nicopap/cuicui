# Widges

We want:

- Bidirectional ECS <-> rust data structure transformation
- Tridirectional? (ECS - rust - Widge tree)
- Actually: Bidirectional ECS <-> (rust data structure , style specification)

## State

Widges can change the state in two ways:

1. Their state is reflection of a global state, and it is changed
2. They emit events and/or run code that changes other state.

## Traits

- `Prefab`: Takes any value of given type and spawns ECS hierarchy based on
  that value
- `ReadWorldValue`: Takes a ECS hierarchy and returns the value of given type
- `ExtractPrefab`: Taxe ECS hierarchy, returns widge tree.

Is this reasonabe? Does it make sense wrt what "We want"?

- Widges should be a sort of `Prefab` with additional information (styling,
  presentation (checkbox? +/- counter? slider? progress bar?), etc)
- Should also be `ReadWorldValue`
- Should be able to query individual values of widge in widge tree, and
  react on change.

## How to define widge tree

Ideal: User has a `struct MenuData`, can get a `QueryPrefab<MenuData>` to access
its values.

`struct MenuData` is just the relevant values.

-> Maybe something like `QueryPrefab<StartGame>`, with some sort of `Changed`
filter? Would trancend the need for events, therefore pushing all logic out of
the struct definition.

```rust
struct AudioMenu {
  master: f32,
  music: f32,
  effects: f32,
}
struct GraphicsMenu {
  vsync: bool,
  msaa: bool,
}
#[derive(Styling)]
#[style(menu, vertical, stretch)]
struct MainMenu {
  #[style(menu)]
  audio: AudioMenu,
  #[style(menu)]
  graphics: GraphicsMenu,
  
}
```

but also:

```kdl
MainMenu {
  width "0.9"; // ( '0.'\d+ | \d+'%' | \d+('px')? ) -> (fraction | percent | pixel ) 
  direction "vertical"; // ( vertical | horizontal )
  space_use "stretch"; // ( stretch | compact )
}
MainMenu.audio {
  // ...
}
AudioMenu {
  // MainMenu.audio applies after AudioMenu. Rules of application is:
  // default -> type specific -> field specific
}
```

--> Probably should support arbitrary additional styling fields, to support
user-defined stuff.

- Should this support css-style selectors? -> Probably not.
- Maybe nested defs?
- "struct", "enum", "f32" node styles?


`HashMap<TypeId, Prefab>`? (see also `chainmap` crate for layered prefabs)

- We said **not `Prefab`**, it should be `HashMap<TypeId, {additional}>` as
  mentioned in "Traits, is this reasonable?"
- Could tell how to present `ReflectRef`
- "additional info" is both a property of container's field and struct.

How would that look?

1. `fn(&dyn Reflect) -> impl Prefab`? What about the reverse? And what about
   hot-reloading?
2. `fn(&dyn Reflect, Styling) -> impl Prefab`, you could define a mapping of
   type -> inner Styling and type -> Styling and pass the relevant styling to
   the function.

At first glance, (2) seems to solve all the problems.

- --> How does `Styling` interacts with `ExtractPrefab`?
- --> Need to track which style node a variable comes from. (not difficult,
      literally only 2 user-defined possible source (maybe 3 if we allow structural
      style definitions))

## Capabilities

A `Prefab` trait that supports:

- **Spawning**: from a `T: Prefab`
- **Deserialization**: If you can get `T: Prefab`, you can spawn it!
- **Querying**: Nesting `Prefab`s within prefabs, and providing efficiency ECS query t
  the nested elements
  - Also querying a value distinct from `T: Prefab`. A slider has much more properties
    than an `f32`, yet `f32` 
- **Change queries**: for example a checkbox, or an "activate" button.
- **Despawning**
- **Serialization**: In addition to querying the associated value, I should be
  able to query the `T: Prefab`.

A `Widge` (trait|struct)? with the following features:

- By default, `Widge` provide a mapping of `(dyn Reflect, ) -> impl Prefab` for
  any `T: Reflect`.
- Also a (`ECS -> (dyn Reflect, Style)`)
- --> this seems incompatible with `Querying` method.

> The following paragraph was an earlier speculation that is now proven to be false:

`Style` doesn't just adjust parameters, but also let users specify which widge
to use for given type. For example, could chose between a plain button or a
checkbox for a `bool`.

> end of erroneous paragraph



## Style as a collection of components

Actually! `Widge`s are controls, logical collection of other `Widge`s, they
exist independently from `Style`/presentation.

In fact, I notice something, all that has to do with style is a component!
So the styling manip can be independent from widgets.

### Problems

- `Widges` are purely logical, how to integrate styles in them?
- Maybe do not make them purely logical, treat them as mix in. The `Widge`
  trait as the glue that makes everything work together.

## `Widge` again

- Leaf widge are units of concern, such as layout, focus, state, graphics.
- Node (or complex) widge are a "mixin" of several widges.

Questions to ask:

(`Counter` is a value with a button for increment and a button for decrement)
(`Label<W>` takes a widget and a text, and extends the selection area of `W`
 to contain both the widget and the text)
(`List<W>` is a dynamically sized list of the same widget with different values)

- What does it mean to mix-in "leaf widges" with `Counter`?
  - Graphics: could do something like an outline
  - focus doesn't mean much, only buttons part of `Counter` should have focus
- What does it mean to mix-in "leaf widges" with `Label<W>`?
  - Graphics: same as before
  - focus: Doesn't mean anything, it is handled by impl of `Label<W>`
    supposedly.
- What does it mean to mix-in "leaf widges" with `List<W>`?
  - Same as before actually.
- `Label<Counter>`?
  - `Counter` isn't focusable. Maybe `Label` should accept only widgets that
    extend `Interact`? `Interact` may be able to delegate entity selection.
- Are there widgets that occupy no space and therefore it is meaningless to
  draw their outline?
  - Nay
- Relationship with ECS? Widge tree, but work in a word assumed hierarchy-free

=> It sounds like graphics (container) could be implemented itself as a widget
that contains another widget?

=> Three concepts depend on widget area & position: outline, selection, layout

```rust
struct Counter<W, V> {
  plus: W,
  minus: W,
  value: V,
}
impl Counter<W: Interact + Widge, V: smts> {
// Counter imaginary impl
fn update_value(
    mut minus: EventReader<Decrement>,
    mut plus: EventReader<Increment>,
    mut value: Mut<Value>,
) {
    let total: i32 = plus.iter().count() - minus.iter().count();
    if total != 0 {
        value.0 += total;
    }
}
}

struct Label<W> {
  text: Text,
  inner: W,
}
// Label imaginary impl
impl Label<W: Interact + Widge> {
fn setup(&self, mut commands, &mut EntityCommands, params: SpawnParams) {
    commands.insert((
      Layout::Foobar,
      InteractArea(todo!()),
    )).with_children(|cmds| {
      self.inner.setup(cmds.spawn_empty(), params.inner);
      self.text.setup(cmds.spawn_empty(),params.text);
    });
}
}
```

Why all those generic parameters?
Suppose widge is object-safe.

```rust
Counter(Label("-", Toggle), todo!(), Label("+", Toggle));
```

Recover widge tree from ECS?

```rust
trait Widge {
  fn read_from_ecs<S: SystemParam>(entity: In<Entity>, params: ) -> Box<Self> where Self: Sized;
  // object-safe methods:
  // ...
}

type WidgeReaderSystem = Box<dyn System<In = Entity, Out = Box<dyn Widge>>>;

#[derive(Default, Resource)]
struct WidgeRegistry {
  readers: HashMap<TypeId, WidgeReaderSystem>,
}
impl AppExtension for App {
  fn register_widge<W: Widge>(&mut self) -> &mut Self {
    let world = &mut self.world;
    if !world.contains_resource::<WidgeRegistry>() {
      world.init_resource::<WidgeRegistry>();
    }
    let mut widge_registry = world.resource_mut::<WidgeRegistry>();
    let mut system: WidgeReaderSystem = Box::new(W::read_from_ecs);
    system.initialize(world);
    widge_registry.readers.insert(TypeId::of::<W>(), system);
    self
  }
}
```

This makes `Box<dyn Widge>` work. Alternative is to add a `WidgePlugin::<W>`
for all new widges, no reliance on object-safety required afterward.

Object-safety was to erase type and limit generic parameters. I guess that
if we need to fully qualify each widget type, the `WidgePlugin` seems a bit
difficult to create.

Concerning type-constructed widget tree, another issue is: How would we read
them from a file? => Maybe type-constructed widget tree is only a constructor,
to modify the ECS so that it has the components we want. Nothing prevents us
to create a second constructor based on a file, that modify the ECS in the same
way.

How to add those `update` systems to the app?

### Widge logic definition

Ideally when defining systems to make widge work, we want to define them without
access to `World`, rather let user pick which subset of `World` to access through
parameters of system. Full access to world is window open to bunch of footguns
(for example, user forgets that several instance of widge may exist at the same
time in `World`)

`WidgeSystemParam` => `SystemParam`, where `WidgeSystemParam` is a collection
of parameters that help reduce world access.

`Widge` could include a concept of "activation"

- Registry of widget systems (similar to `WidgetReaderSystem`, but for updates)
- An inherited component similar to visibility `Activated(bool)`
- We have N instance of Widget, with given system, we should run system once
  per instance?
  - Or maybe we can pass a list of entities?
  - I need to figure the feasability of various WidgeSystemParam

Consider this:

- Separate _data_ and _behavior_.
  - `Counter` may be something else than a widget. It could work as a special
    system that links several widgets together.
  - If I want to serialize it, I somehow need to describe it as data.

### Attack plan

- [ ] Define a series of logical widges, systems are classic bevy systems
- [ ] Find a way to style the widges, probably using a TypeId