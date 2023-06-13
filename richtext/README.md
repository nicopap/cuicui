# Rich text

A rich text component for bevy.

https://github.com/nicopap/cuicui/assets/26321040/e81b2dae-1dda-4188-ace1-6c2a8316c90c

The current bevy `Text` component extremely primitive, forcing you to do a bunch
of error-prone operations to interact with it.

`cuicui_richtext` gives you all you need to display fancy text on screen.
With `cuicui_richtext` you can:

- When spawning the `Text` component, refer directly to fields in `Reflect`
  components and resources to display and update them in-line. No need to
  manually update them afterward!
- Declare styling inline, rather than through code.
- Not have to worry about `TextSection`s at all.
- Set text value by name rather than by index.
- And _wayyy too much_ more!

I won't introduce individually each feature. Rather, the rest of this README is
a tutorial explaining how to use `cuicui_richtext`.

## Showing your character stats

Rich text, as the name implies, it is a library to write text on screen.

Consider your classic RPG character:

```rust
#[derive(Component)]
struct Player;

#[derive(Component)]
struct Stats {
    mana: i32,
    health: i32,
    defense: i32,
}
```

Using the primitive bevy UI system, you would have to construct a stat menu,
give each variables its own section (manually) and then set those values
appropriately:

<details><summary><b>See how you would define the stats menu in bevy</b></summary>

```rust
use bevy::prelude::*;

#[derive(Component)]
struct MenuText;

fn spawn_menu(
    mut commads: Commands,
    player: Query<&Stats, With<Player>>,
    assets: Res<AsssetServer>,
) {
    let stats = player.single();
    
    let base_style = TextStyle { font: assets.load("stats_menu_font.ttf"), .. default() };
    let mana_style = TextStyle { color: Color::PURPLE, .. default() };
    let health_style = TextStyle { color: Color::RED, .. default() };
    let defense_style = TextStyle { color: Color::BLUE,  .. default() };
    commands.spawn(TextBundle::from_sections([
        TextSection::new("Player stats:\n------------", base_style),
        TextSection::new("\nHealth:", health_style.clone()),
        TextSection::new(stats.health.to_string(), health_style),

        TextSection::new("\nDefense:", defense_style.clone()),
        TextSection::new(stats.defense.to_string(), defense_style),

        TextSection::new("\nMana:", mana_style.clone()),
        TextSection::new(stats.mana.to_string(), mana_style),
    ])).insert(MenuText);
}
```

Supposing you want to update the menu in real time, you would need an additional
system:

```rust
fn update_stat_menu(
    mut menu: Query<&mut Text, With<MenuText>>,
    player: Query<&Stats, With<Player>>,
) {
    let mut text = menu.single_mut();
    let stats = player.single();

    text.sections[2].value.clear();
    text.sections[4].value.clear();
    text.sections[6].value.clear();

    write!(&mut text.sections[2].value, "{}", &stats.health);
    write!(&mut text.sections[4].value, "{}", &stats.defense);
    write!(&mut text.sections[6].value, "{}", &stats.mana);
}
```

Here we use `clear` followed by `write!` to avoid extra allocations.

We index `sections` of the `Text` component to access the bit of text we want
to edit. We could replace the magic values `2`, `4` and `6` by constants such
as `const HEALTH_TEXT_SECTION: usize = 2` etc. But you still have to keep track
of the index when _spawning_ the text component. And also, this is far more
code.

TODO: screenshot

</details>

You'll notice it's a bit verbose, and error-prone.

### With `cuicui_richtext`

Now let's rewrite it using cuicui_richtext.

First, we need add the `MinRichTextPlugin`:

```rust
fn main() {
    app
        // ... default plugins whatever man ...
        .add_plugin(MinRichTextPlugin)
        // ... your systems and stuff ...
}
```

Then, time to rewrite how we spawn our menu text:

```rust
const MENU_FORMAT_STRING: &str = "\
Player stats
------------
Health: {health}
Defense: {defense}
Mana: {mana}";

#[derive(Resource)]
struct Fonts {
    stats_menu: Handle<Font>,
}

fn spawn_menu(mut commads: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(Fonts {
        stats_menu: assets.load("stats_menu_font.ttf"),
    });
    commands.spawn(MakeRichText::new(MENU_FORMAT_STRING));
}
fn update_stat_menu(mut bindings: WorldBindingsMut, player: Query<&Stats, With<Player>>) {
    let stats = player.single();

    bindings.set_content("health", &stats.health);
    bindings.set_content("defense", &stats.defense);
    bindings.set_content("mana", &stats.mana);
}
```

TODO: screenshot

<details><summary><b>Click here for a detailed explanation on how `MENU_FORMAT_STRING` becomes Text</b></summary>

Let's take a closer to `MENU_FORMAT_STRING`, without the rust syntax:

```
Player stats
------------
Health: {health}
Defense: {defense}
Mana: {mana}
```

`cuicui_richtext` splits this string in the following `TextSection`s:

```c
"Player stats\n------------\nHealth: "
"{health}"
"\nDefense: "
"{defense}"
"\nMana: "
"{mana}"
```

The sections within braces `{}` are special. Those are *bindings*.

Those sections can then be referenced by the name within braces.

`WorldBindings` is a collection of `binding_name: value` pairs.
You use it to set the value of a binding.

Alternatively, you can set the `RichText` binding directly. This will limit
the *binding* to the `RichText` of the same entity.

```rust
fn bind_stuff(mut bindings: WorldBindingsMut, mut rich_texts: Query<RichText>) {
    world_bindings.set_content("health", &41);

    for mut rich_text in rich_texts.iter_mut() {
        rich_text.set_content("mana", &32);
    }
}
```

> **Note**
> Notice how we use the `set_content` method, see more in the
> section about *modifiers*.

Then a system will read the `WorldBindings` and `RichText` components,
and update all the `Text` spawned with `MakeRichText` according to the *bindings*.

Note that the `TextSection` is only updated whend the *binding* value is updated.

</details>

### Using reflection to let `cuicui_richtext` care about reading the components

Well… Actually, while we removed the need to query for the specific entity with
the `Text` component and manually set each section, we still need to manually
set the value of the `health`, `defense` and `mana` *bindings*.

We now run the risk of making spelling mistakes, forgetting to update new stats
when we add them, and it's still some code to write.
`cuicui_richtext` can do better, far better.

`cuicui_richtext` can use reflection to automatically update *bindings* without
having to write any more code.

First, let's derive `Reflect` on the components we read:

```rust
#[derive(Component, Reflect)]
#[reflect(Component, Queryable)]
struct Player;

#[derive(Component, Reflect)]
#[reflect(Component, Queryable)]
struct Stats {
    // ...
}
fn main() {
    app
        .add_plugins(DefaultPlugings)
        // ... we also need to register them!
        .register_type::<Player>()
        .register_type::<Stats>()
        // ...
}
```

Now we can use the *binding source* syntax in the `MENU_FORMAT_STRING`:

```rust
const MENU_FORMAT_STRING: &str = "\
Player stats
------------
Health: {Marked(Player).Stats.health:}
Defense: {Marked(Player).Stats.defense:}
Mana: {Marked(Player).Stats.mana:}";
```

That's all we need to do, now we can *delete the `update_stat_menu` system and everything
is taken care of*.

<details><summary><b>Click here to learn how the binding source syntax works</b></summary>

With the *source binding* syntax, the format string looks as follow:

```
Player stats
------------
Health: {Marked(Player).Stats.health:}
Defense: {Marked(Player).Stats.defense:}
Mana: {Marked(Player).Stats.mana:}
```

The bindings are now: `{Marked(Player).Stats.field:}`, let's split this into its
fundamental components:

- *source*: `Marked(Player).Stats.field`, let's split this even more:
    - *query*: `Marked(Player)`: Select an entity based on a marker component
    - *type*: `.Stats`: the type of the component we want to read
    - *field*: `.field`: a [reflection path] used to access the field we care about
- *format parameters*: (`:`, the colon) the rust [formatting parameters] used to turn
  the value in `field` into text.

Visually:

```
  source
  ----------------------------
  query          type   field  format parameter
  -------------- ------ ------ --
{ Marked(Player) .Stats .field : }
```

`MinRichTextPlugin` takes the *source bindings* and creates a `Hook` per *source binding*.
The `Hook` is a bit of code that tells `MinRichTextPlugin` the following:

- What to read from the ECS. For example, in this case, it is "the field `field`
  of component `Stats` of entity marked with `Player`
- How to translate this into a *modifier* (more on this later). In this case,
  we tell it to use `fmt::Display` and set the text value of the section to
  the format result.
- What *binding* to set that result to. This is the text between braces:
  "Marked(Player).Stats.field:" 

The code only runs if the component in question has been updated since last time.

#### Kind of query

`cuicui_richtext` has several *queries* you can chose from:

| query         | example                          | matches | reads |
|---------------|----------------------------------|---------|-------|
| `Res(<type>)` | `Res(SomeResource).path.to.field` | `Resource` with type `SomeResource` | The same resource |
| `One(<type>)` | `One(PlayerStats).path.to.field` | An `Entity` with `PlayerStats`, fails if more than one entity has the `PlayerStats` component | The same component |
| `Name(value)` | `Name(Player).Stats.path.to.field` | The first `Entity` encountered with the [`Name`] component which value is "Player" | The `Stats` component |
| `Marked(<type>)` | `Marked(Player).Stats.path.to.field` | An `Entity` with `Player`, fails if more than one entity has the `Player` component | The `Stats` component |


| ❗ **The `Name` query iterates over all entities repetitively if none has the give name** ❗ |
|----------------------------------------------------------------------------------------------|

| ❗ **You must `#[reflect(Queryable)]` components you wish to access with binding sources** ❗ |
|-----------------------------------------------------------------------------------------------|


[reflection path]: https://docs.rs/bevy/latest/bevy/reflect/trait.GetPath.html#syntax
[formatting parameters]: https://doc.rust-lang.org/stable/std/fmt/index.html#formatting-parameters
[`Name`]: https://docs.rs/bevy/latest/bevy/core/struct.Name.html

</details>


### Styling

Well… we don't handle styling. It is imperative that our UI has
nice and appealing colors! We aren't making a TIS-100 clone!

Turns out, styling is the strong suit of `cuicui_richtext`.

A section in `cuicui_richtext` is not just text, it's also each individual
style parameters a `TextStyle` has (ie: color and size, yeah not that much).

To set the color of a section to red in `cuicui_richtext`, you would do as follow:

```
{ Color: Red |Bloody Text!}
```

TODO: screenshot

The bit between the `|` and the closing `}` is just some text, it works exactly
like the rest of a format string. The only difference is that the text will be
red.

You can have several of them:

```
{Color: grey|Lucy Westenra}: {Color:blue |You look pale, mister}
{Color: grey|Dracula}: {Color: red|I will drink your blood!}
```

TODO: screenshot

And you can nest them, including *bindings*:

```
{Color: grey|Lucy Westenra}: {Color:blue|
    {Color:grey|Dracula} said to me: "{Color: red|I will drink your {fluid}!}"
}
```

`cuicui_richtext` will intelligently split the string in one `TextSection` per
individual section of text, and assign them the correct style values.

Let's rewrite our `MENU_FORMAT_STRING` to replicate the styling we used in
the initial bevy example:

```rust
const MENU_FORMAT_STRING: &str = "\
{ Font: stats_menu_font.ttf |\
Player stats
------------
{Color:Red    |Health: {Marked(Player).Stats.health:}}
{Color:Blue   |Defense: {Marked(Player).Stats.defense:}}
{Color:Purple |Mana: {Marked(Player).Stats.mana:}}\
}";
```

TODO: screenshot

<details><summary><b>Click here to learn more about modifiers</b></summary>

In `{ Color: Red |Bloody Text!}`, `Color` is a *modifier*.

*Modifiers*, as the name implies, *modifies* a set of fields.

In fact, in `cuicui_richtext`, *everything is a modifier*. Including text!

| Modifier | value | what it does |
|----------|-------|--------------|
| `Color`  | css color string | Sets the color of all text within to provided value|
|`ShiftHue`| float | Sets the color of all text within to the parent's text color plus given shift in hue (over 360º)|
| `RelSize`| float | Multiplies the font size of all text within by provided value|
| `Content`| text  | Set the text of all sections within to provided value|
| `Font`   | file path | Set the font of all text within to provided value. `file path` must be loaded first through `AssetServer`|

Notice `Content`. Sound familiar? Yeah, that's because text by default is just
the `Content` *modifier*:

```
Those two lines are actually identical
{Content: Those two lines are actually identical}
```

The text is converted to a `Content` *modifier* that only applies to the first
section of the section between `{}`.

#### Nesting

Let's take another format string example:

```
Some text {Color: green|that is green {Font:bold.ttf|and bold {RelSize:3.0|and big}} at the same time}.
```

Sections within other sections are flattened into a single flat list.
Each section inherits the *modifiers* from their parent and apply their own
afterward.

The previous format string would be split in **six** segments as follow:

```
Some text ┆that is green ┆and bold ┆and big┆ at the same time┆.
     ↑    ┆     ↑        ┆    ↑    ┆   ↑   ┆     ↑           ┆↑
     │          │             │        │    root + green      root formatting
     │          │             │     root + green + bold.ttf font + size×3
     │     root + green   root + green + bold.ttf font
root formatting
```

</details>


### Dynamic styling

This is a bore, all this text is static. Sure, we have a nice markup,
but we want moving rainbow, dancing letters, singing words!

I've yet to see how you made that first video.

Sure let me tell you.

Not only can text be set through *binding*, but actually any *modifier* can.

Let's go back to our character stats screen. Say our character is actually
**an unicorn**. This means, of course, that we need to cycle through the whole
color wheel for all the menu text (appart from the stats name and values).

Remember we have been using all this time the `MinRichTextPlugin`.
We need to upgrade to `RichTextPlugin` to set dynamically *modifiers* that
span more than a single section.

Concerning our *format string*, we need to introduce a binding for the `Color`
modifier on the whole text:

```diff
 fn main() {
     app
-        .add_plugin(MinRichTextPlugin)
+        .add_plugin(RichTextPlugin)
 }
 
 // ...
 
 const MENU_FORMAT_STRING: &str = "\
-{ Font: stats_menu_font.ttf |\
+{ Font: stats_menu_font.ttf, Color: {color} |\
 Player stats
 ------------
 {Color:Red    |Health: {Marked(Player).Stats.health:}}
 {Color:Blue   |Defense: {Marked(Player).Stats.defense:}}
 {Color:Purple |Mana: {Marked(Player).Stats.mana:}}\
 }";
```

Now, we have a `color` binding. We can update it by accessing the `WorldBindingsMut`
resource:

```rust
fn update_color_system(mut bindings: WorldBindingsMut, time: Res<Time>) {
    let hue = time.seconds_since_startup() % 360.0;
    let color = Color::hsl(hue, 0.9, 0.9);
    bindings.set("color", color.into());
}
```

## TODO: aliases and chops

## A dialog system in bevy

> **Warning**
> TODO: complete this section when context fields land.
>
> TODO: This is false until we do Entity as section.

`cuicui_richtext` can also make your dialog's text more dynamic.
Very much like the [febucci Unity plugin], `cucui_richtext` has a set of primitives to give
spice to text. This includes changing the transform, color, oppacity, visibility
of individual characters or words, in sync or other.

[febucci Unity plugin]: https://www.febucci.com/text-animator-unity/docs/installing-and-quick-start/

## Previous work

- [**bevy_ui_bits**][bui_bits]

[bui_bits]: https://github.com/septum/bevy_ui_bits
[`fmt`]: https://doc.rust-lang.org/stable/std/fmt/index.html
[`Color`]: https://docs.rs/bevy/latest/bevy/prelude/enum.Color.html
[fs-grammar]: https://github.com/nicopap/cuicui/blob/main/design_doc/richtext/informal_grammar.md
[docsrs-root]: https://docs.rs/cuicui_richtext/latest/cuicui_richtext/

## TODO

- [ ] fab_derive: Split the path detection code in a different crate.
- [ ] all: design feature gates to avoid compiling stuff not used.
- [ ] fab_parse: performance: use jagged array for `tree::Sections` to avoid insane amount of alloc
- [ ] fab_derive: Document `impl_modify` macro fully. Specifically: settle on a naming convention
      and use it consistently.
- [ ] fab_derive: Test `impl_modify` more thourougfully
- [ ] fab resolve: Verify validaty of multiple write fields
- [ ] fab resolve + fab_derive: Context field access tracking (implemented, not tested)
- [ ] fab resolve: Test MinResolver
- [ ] bevy_fab trackers: Test the reflection-component-based trackers
- [ ] fab parse: review the informal_grammar.md file
- [ ] richtext: Text2d support
- [ ] richtext: Modify a Vec<&mut Text> over TextSections, to allow all kind of effects
- [ ] fab_parse: Consider using a `View<Box<Fn(&mut Style)>>` for styling
    - This would make it very much like bindings, which is cool
    - Would allow local styles, which is more sensible than having to define them not where they are used.
- [ ] everything: Document the hell out of everything
- [ ] bevy_fab: context-specific Resolvers. Could use a different resolver depending on the text
      being created, still sharing the same interner and Modify (though this conflicts with Resolver
      as a Modify associated type).

### Questionable/very difficult

- [ ] fab_parse: performance: Directly intern in parser, makes comparisons in post_process faster.
- [ ] bevy_fab: Add way to register "Send event" as `formatter` (this doesn't make sense as a formatter)
- [ ] fab parse: test and improve error messages
- [ ] bevy_fab trackers: Check that the target field changed before updating binding
- [ ] richtext parser: Allow compile-time verification of rich text spec through a proc macro
- [ ] fab resolve: Handle binding that depends on fields (Option<Modifier> in binding view)
    -> Problem: Requires clone + updating it is non-trivial

