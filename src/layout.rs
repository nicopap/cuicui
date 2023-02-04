//! Layouting system for bevy cuicui.
//!
//! The layouting system is very dumb. It is based on [`Container`]s.
//! A [`Container`] layouts its content in either a column or a row.
//!     
//! The individual items are positioned according to one of two possible [`SpaceUse`].
//! Either items are compactly put one after another with [`SpaceUse::Compact`],
//! or they [`SpaceUse::Stretch`] to the parent's Vertical/Horizontal space.
//!
//! If you want some margin, use [`Spacer`].
//! The [`Spacer::parent_ratio`] is the percent of the containing container's
//! total size.
//!
//! All things in a cuicui layout has a known fixed size. This is why
//! everything needs to live in a root countainer of a fixed size.
//!
//! ## Things you can't do
//!
//! * Several `SpaceUse::Stretch` vertical layout within a vertical layout (same for horizontal)
//!   A single `SpaceUse::Stretch` is accepted, but several do not make sense.
//! * Note that this is transitive, so a `Stretch` vertical layout within
//!   an horizontal layout within a `Stretch` vertical layout is also a no-no.
//! * `Spacer` within a `SpaceUse::Compact`.
//!
//! ## TODO:
//!
//! Additionally, `Node::Spacer` and `Node::Static` can have a single
//! `Node` child. Usually a `Node::Container`. Not sure what it means yet.
//!
//! This will be replaced with container of parent% and set sizes.
use std::fmt;

use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;

mod error;

#[derive(Clone, Default, Copy, PartialEq)]
#[cfg_attr(feature = "reflect", derive(Reflect, FromReflect))]
pub struct Pos {
    pub left: f32,
    pub top: f32,
}
impl Pos {
    fn set_cross(&mut self, direction: Direction, cross: f32) {
        match direction {
            Direction::Vertical => self.left = cross,
            Direction::Horizontal => self.top = cross,
        }
    }
    fn set_axis(&mut self, direction: Direction, axis: f32) {
        match direction {
            Direction::Vertical => self.top = axis,
            Direction::Horizontal => self.left = axis,
        }
    }
}
#[derive(Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "reflect", derive(Reflect, FromReflect))]
pub struct Size {
    pub width: f32,
    pub height: f32,
}
impl Size {
    fn with(direction: Direction, axis: f32, cross: f32) -> Self {
        match direction {
            Direction::Vertical => Self { height: axis, width: cross },
            Direction::Horizontal => Self { height: cross, width: axis },
        }
    }
    fn on(&self, direction: Direction) -> f32 {
        match direction {
            Direction::Vertical => self.height,
            Direction::Horizontal => self.width,
        }
    }
    fn cross(&self, direction: Direction) -> f32 {
        match direction {
            Direction::Vertical => self.width,
            Direction::Horizontal => self.height,
        }
    }
}
impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}×{}", self.width, self.height)
    }
}
type Bound = Result<f32, Entity>;
#[derive(Clone, Copy, PartialEq)]
struct Bounds {
    width: Bound,
    height: Bound,
}
impl Bounds {
    fn on(
        &self,
        direction: Direction,
        this: Entity,
        names: &Query<&Name>,
    ) -> Result<f32, error::Why> {
        match direction {
            Direction::Vertical => self
                .height
                .map_err(|e| error::parent_is_stretch("height", this, e, names)),
            Direction::Horizontal => self
                .width
                .map_err(|e| error::parent_is_stretch("width", this, e, names)),
        }
    }
    fn undefine(&self, dir: Direction, on: Entity) -> Self {
        match dir {
            Direction::Horizontal => Self { width: Err(on), ..*self },
            Direction::Vertical => Self { height: Err(on), ..*self },
        }
    }
}
impl fmt::Display for Bounds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.width {
            Ok(v) => write!(f, "{v}×")?,
            Err(_) => write!(f, "?×")?,
        }
        match self.height {
            Ok(v) => write!(f, "{v}"),
            Err(_) => write!(f, "?"),
        }
    }
}
impl From<Size> for Bounds {
    fn from(value: Size) -> Self {
        Self { width: Ok(value.width), height: Ok(value.height) }
    }
}

/// Position and size of a [`Node`] as computed by the layouting algo.
///
/// Note that `Pos` will always be relative to the top left position of the
/// containing node.
#[derive(Component, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "reflect", derive(Reflect, FromReflect))]
pub struct PosRect {
    size: Size,
    pos: Pos,
}
impl PosRect {
    pub fn pos(&self) -> Pos {
        self.pos
    }
    pub fn size(&self) -> Size {
        self.size
    }
}

#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(feature = "reflect", derive(Reflect, FromReflect))]
pub struct Container {
    pub direction: Direction,
    pub space_use: SpaceUse,
}
#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "reflect", derive(Reflect, FromReflect))]
pub enum Direction {
    Vertical,
    Horizontal,
}
impl Direction {
    fn perp(self) -> Self {
        match self {
            Direction::Vertical => Direction::Horizontal,
            Direction::Horizontal => Direction::Vertical,
        }
    }
    fn size_name(&self) -> &'static str {
        match self {
            Direction::Vertical => "height",
            Direction::Horizontal => "width",
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(feature = "reflect", derive(Reflect, FromReflect))]
pub enum SpaceUse {
    Stretch,
    Compact,
}
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(feature = "reflect", derive(Reflect, FromReflect))]
pub struct Spacer {
    pub parent_ratio: f32,
}
#[derive(Component)]
#[cfg_attr(feature = "reflect", derive(Reflect, FromReflect))]
pub enum Node {
    Container(Container),
    Spacer(Spacer),
    Static(Size),
}

#[derive(Component)]
#[cfg_attr(feature = "reflect", derive(Reflect, FromReflect))]
pub struct Root {
    pub container: Container,
    pub bounds: Size,
}

#[derive(Clone, Copy, PartialEq)]
struct AtOutput {
    axis: f32,
    cross: f32,
}
impl Container {
    fn layout(
        &self,
        this: Entity,
        children: &Children,
        bounds: Bounds,
        to_update: &mut Query<&mut PosRect>,
        nodes: &Query<(Entity, (&Node, &Children))>,
        names: &Query<&Name>,
    ) -> Result<Size, error::Why> {
        use SpaceUse::*;
        let Self { direction: dir, space_use } = *self;
        if children.is_empty() {
            return Ok(Size::default());
        }
        let mut children_size = 0.0;
        let mut cross_max = 0.0_f32;
        // TODO: check we do not have multiple Stretch going in same direction
        // recursively in this.
        // Currently, it just gives full length to the first one.
        // TODO: also if there is only one, make sure to first comute all
        // the other layouts before computing this one.
        // TODO: check for Spacer (or %) in Compact containers. I guess it makes
        // sense technically.
        let mut node_children_count = 0;
        let child_bound = match space_use {
            Stretch => bounds.undefine(dir.perp(), this),
            Compact => bounds.undefine(dir.perp(), this).undefine(dir, this),
        };
        for child in nodes.iter_many(children) {
            let result = layout_at(child, dir, child_bound, to_update, nodes, names)?;
            children_size += result.axis;
            cross_max = cross_max.max(result.cross);
            node_children_count += 1;
        }
        match space_use {
            Stretch => {
                let total_space_between = bounds.on(dir, this, names)? - children_size;
                if total_space_between < 0.0 {
                    let name = names
                        .get(this)
                        .map_or_else(|_| format!("{this:?}"), |n| n.to_string());
                    let n = children.len();
                    let dir_name = dir.size_name();
                    panic!(
                        "Yo container {name} of size {bounds} contains more stuff than it possibly can!\n\
                         You gotta either make it larger or reduce the size of things within it.\n\
                         It has exactly {n} items for a total {dir_name} of {children_size}."
                    );
                }
                let space_between = total_space_between / (node_children_count - 1) as f32;
                let mut iter = to_update.iter_many_mut(children);
                let mut axis_offset = 0.0;
                while let Some(mut space) = iter.fetch_next() {
                    space.pos.set_axis(dir, axis_offset);
                    let offset = (cross_max - space.size.cross(dir)) / 2.0;
                    space.pos.set_cross(dir, offset);
                    axis_offset += space.size.on(dir) + space_between;
                }
                Ok(Size::with(dir, bounds.on(dir, this, names)?, cross_max))
            }
            Compact => {
                let mut axis_offset = 0.0;
                let mut iter = to_update.iter_many_mut(children);
                while let Some(mut space) = iter.fetch_next() {
                    space.pos.set_axis(dir, axis_offset);
                    space.pos.set_cross(dir, 0.0);
                    axis_offset += space.size.on(dir);
                }
                Ok(Size::with(dir, children_size, cross_max))
            }
        }
    }
}
// This functions' responsability is to compute the layout for `current` Entity,
// and all its children.
//
// Rules for this function:
//
// - Nodes will set **their own size** with the `to_update` query.
// - **the position of the children** will be set with `to_update`.
fn layout_at(
    (this, node): (Entity, (&Node, &Children)),
    parent_dir: Direction,
    bounds: Bounds,
    to_update: &mut Query<&mut PosRect>,
    nodes: &Query<(Entity, (&Node, &Children))>,
    names: &Query<&Name>,
) -> Result<AtOutput, error::Why> {
    Ok(match node {
        (Node::Container(container), children) => {
            let size = container.layout(this, children, bounds, to_update, nodes, names)?;
            if let Ok(mut to_update) = to_update.get_mut(this) {
                to_update.size = size;
            }
            AtOutput {
                axis: size.on(parent_dir),
                cross: size.cross(parent_dir),
            }
        }
        (Node::Spacer(spacer), _) => {
            let axis = bounds.on(parent_dir, this, names)? * spacer.parent_ratio;
            let size = Size::with(parent_dir, axis, 0.0);
            if let Ok(mut to_update) = to_update.get_mut(this) {
                to_update.size = size;
            }
            AtOutput { axis, cross: 0.0 }
        }
        (Node::Static(size), _) => {
            if let Ok(mut to_update) = to_update.get_mut(this) {
                to_update.size = *size;
            }
            AtOutput {
                axis: size.on(parent_dir),
                cross: size.cross(parent_dir),
            }
        }
    })
}
// TODO:
// - minimize recomputation using `Changed`
// - better error handling (log::error!)
// - maybe parallelize
/// Run the layout algorithm on
#[sysfail(log(level = "error"))]
pub fn compute_layout(
    mut to_update: Query<&mut PosRect>,
    nodes: Query<(Entity, (&Node, &Children))>,
    names: Query<&Name>,
    roots: Query<(Entity, &Root, &Children)>,
) -> anyhow::Result<()> {
    for (entity, Root { container, bounds }, children) in &roots {
        if let Ok(mut to_update) = to_update.get_mut(entity) {
            to_update.size = *bounds;
        }
        let bounds = Bounds::from(*bounds);
        container.layout(entity, children, bounds, &mut to_update, &nodes, &names)?;
    }
    Ok(())
}
/// Update transform of things that have a `PosRect` component.
pub fn update_transforms(mut positioned: Query<(&PosRect, &mut Transform), Changed<PosRect>>) {
    for (pos, mut transform) in &mut positioned {
        transform.translation.x = pos.pos.left;
        transform.translation.y = pos.pos.top;
    }
}

#[derive(SystemLabel)]
pub enum Systems {
    ComputeLayout,
}

pub struct Plug;
impl Plugin for Plug {
    fn build(&self, app: &mut App) {
        app.add_system(compute_layout.label(Systems::ComputeLayout));

        #[cfg(feature = "reflect")]
        app.register_type::<Container>()
            .register_type::<Direction>()
            .register_type::<Node>()
            .register_type::<Pos>()
            .register_type::<PosRect>()
            .register_type::<Root>()
            .register_type::<Size>()
            .register_type::<Spacer>()
            .register_type::<SpaceUse>();
    }
}
