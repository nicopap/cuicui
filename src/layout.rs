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
//! Currently all layout's cross-axis is aligned to the top or left.
//! This is temporary, The intention is that all layouts are centered on the cross-axis.
//! If you want stuff to be aligned on the cross-axis, actually just use
//! an horizontal layout instead of vertical and vis-versa.
//!
//! Additionally, `Node::Spacer` and `Node::Static` can have a single
//! `Node` child. Usually a `Node::Container`. Not sure what it means yet.
//!
//! This will be replaced with container of parent% and set sizes.
use std::fmt;

use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub struct Pos {
    pub left: f32,
    pub top: f32,
}
impl Pos {
    fn set_cross(&mut self, direction: Direction, cross: f32) {
        match direction {
            Direction::Vertical => self.top = cross,
            Direction::Horizontal => self.left = cross,
        }
    }
    fn set_axis(&mut self, direction: Direction, axis: f32) {
        match direction {
            Direction::Vertical => self.left = axis,
            Direction::Horizontal => self.top = axis,
        }
    }
}
#[derive(Clone, Copy, Default, PartialEq)]
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
    fn with_on(self, direction: Direction, axis: f32) -> Self {
        match direction {
            Direction::Vertical => Self { height: axis, ..self },
            Direction::Horizontal => Self { width: axis, ..self },
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
/// Position and size of a [`Node`] as computed by the layouting algo.
///
/// Note that `Pos` will always be relative to the top left position of the
/// containing node.
#[derive(Component, Clone, Copy, PartialEq)]
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
pub struct Container {
    pub direction: Direction,
    pub space_use: SpaceUse,
}
#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    Vertical,
    Horizontal,
}
impl Direction {
    fn size_name(&self) -> &'static str {
        match self {
            Direction::Vertical => "height",
            Direction::Horizontal => "width",
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
pub enum SpaceUse {
    Stretch,
    Compact,
}
#[derive(Clone, Copy, PartialEq)]
pub struct Spacer {
    pub parent_ratio: f32,
}
#[derive(Component)]
pub enum Node {
    Container(Container),
    Spacer(Spacer),
    Static(Size),
}

#[derive(Component)]
pub struct Root {
    pub container: Container,
    pub bounds: Size,
}

#[derive(Clone, Copy, PartialEq)]
struct AtOutput {
    axe_offset: f32,
    cross_size: f32,
}
impl Container {
    fn layout(
        &self,
        current: Entity,
        children: &Children,
        bounds: Size,
        to_update: &mut Query<&mut PosRect>,
        nodes: &Query<(Entity, (&Node, &Children))>,
        names: &Query<&Name>,
    ) -> Size {
        use SpaceUse::*;
        let Self { direction: node_dir, space_use } = *self;
        if children.is_empty() {
            return Size::default();
        }
        let mut children_size = 0.0;
        let mut cross_max = 0.0_f32;
        for child in nodes.iter_many(children) {
            let result = layout_at(child, node_dir, bounds, to_update, nodes, names);
            children_size += result.axe_offset;
            cross_max = cross_max.max(result.cross_size);
        }
        let size = match space_use {
            Stretch => {
                let total_space_between = bounds.on(node_dir) - children_size;
                if total_space_between < 0.0 {
                    let name = names
                        .get(current)
                        .map_or(format!("{current:?}"), |n| n.to_string());
                    let n = children.len();
                    let dir_name = node_dir.size_name();
                    panic!(
                        "Yo container {name} of size {bounds} contains more \
                            stuff than it possibly can!\n\
                            You gotta either make it larger or reduce the size \
                            of things within it.\n\
                            It has exactly {n} items for a total {dir_name} \
                            of {children_size}."
                    );
                }
                let space_between = total_space_between / (children.len() - 1) as f32;
                let mut iter = to_update.iter_many_mut(children);
                let mut axis_offset = 0.0;
                while let Some(mut space) = iter.fetch_next() {
                    space.pos.set_axis(node_dir, axis_offset);
                    // TODO: centering (should be (bounds.cross(node_dir) - space.size.cross(node_dir)) / 2.0)
                    space.pos.set_cross(node_dir, 0.0);
                    axis_offset += space.size.on(node_dir) + space_between;
                }
                Size::with(node_dir, bounds.on(node_dir), cross_max)
            }
            Compact => {
                let mut axis_offset = 0.0;
                let mut iter = to_update.iter_many_mut(children);
                while let Some(mut space) = iter.fetch_next() {
                    space.pos.set_axis(node_dir, axis_offset);
                    space.pos.set_cross(node_dir, 0.0);
                    axis_offset += space.size.on(node_dir);
                }
                Size::with(node_dir, children_size, cross_max)
            }
        };
        if let Ok(mut to_update) = to_update.get_mut(current) {
            to_update.size = size;
        }
        size
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
    (current, node): (Entity, (&Node, &Children)),
    parent_dir: Direction,
    bounds: Size,
    to_update: &mut Query<&mut PosRect>,
    nodes: &Query<(Entity, (&Node, &Children))>,
    names: &Query<&Name>,
) -> AtOutput {
    match node {
        (Node::Container(container), children) => {
            let size = container.layout(current, children, bounds, to_update, nodes, names);
            AtOutput {
                axe_offset: size.on(parent_dir),
                cross_size: size.cross(parent_dir),
            }
        }
        (Node::Spacer(spacer), children) => {
            let axe_offset = bounds.on(parent_dir) * spacer.parent_ratio;
            let inner_bounds = bounds.with_on(parent_dir, axe_offset);
            let cross_size = if let Some(child) = nodes.iter_many(children).next() {
                let result = layout_at(child, parent_dir, inner_bounds, to_update, nodes, names);
                // TODO: set child position
                result.cross_size
            } else {
                0.0
            };
            let size = Size::with(parent_dir, axe_offset, cross_size);
            if let Ok(mut to_update) = to_update.get_mut(current) {
                to_update.size = size;
            }
            AtOutput { axe_offset, cross_size }
        }
        (Node::Static(size), children) => {
            if let Some(child) = nodes.iter_many(children).next() {
                let result = layout_at(child, parent_dir, *size, to_update, nodes, names);
                // TODO: set child position
            }
            if let Ok(mut to_update) = to_update.get_mut(current) {
                to_update.size = *size;
            }
            AtOutput {
                axe_offset: size.on(parent_dir),
                cross_size: size.cross(parent_dir),
            }
        }
    }
}
pub fn compute_layout(
    mut to_update: Query<&mut PosRect>,
    nodes: Query<(Entity, (&Node, &Children))>,
    names: Query<&Name>,
    roots: Query<(Entity, &Root, &Children)>,
) {
    for (entity, Root { container, bounds }, children) in &roots {
        container.layout(entity, children, *bounds, &mut to_update, &nodes, &names);
    }
}
pub fn update_transforms(mut positioned: Query<(&PosRect, &mut Transform), Changed<PosRect>>) {}

#[derive(SystemLabel)]
pub enum Systems {
    ComputeLayout,
}

pub struct Plug;
impl Plugin for Plug {
    fn build(&self, app: &mut App) {
        app.add_system(compute_layout.label(Systems::ComputeLayout));
    }
}
