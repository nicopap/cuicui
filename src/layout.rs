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
//! Total size.
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
//! Unlike `Flexbox` when something doesn't make intuitive sense, cuicui tells
//! the user that something is ambiguous and helps them have a clear layout
//! that directly maps to something you could picture in your head.
//!
//! ## TODO:
//!
//! Currently all layout's cross-axis is aligned to the top or left.
//! This is temporary, The intention is that all layouts are centered on the cross-axis.
//! If you want stuff to be aligned on the cross-axis, actually just use
//! an horizontal layout instead of vertical and vis-versa.

use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq)]
struct Pos {
    left: f32,
    top: f32,
}
impl Pos {
    pub(crate) fn set_cross(&mut self, direction: Direction, cross: f32) {
        match direction {
            Direction::Vertical => self.top = cross,
            Direction::Horizontal => self.left = cross,
        }
    }
    pub(crate) fn set_axis(&mut self, direction: Direction, axis: f32) {
        match direction {
            Direction::Vertical => self.left = axis,
            Direction::Horizontal => self.top = axis,
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
struct Size {
    width: f32,
    height: f32,
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
/// Position and size of a [`Node`] as computed by the layouting algo.
///
/// Note that `Pos` will always be relative to the top left position of the
/// containing node.
#[derive(Component, Clone, Copy, PartialEq)]
struct PosRect {
    size: Size,
    pos: Pos,
}
impl PosRect {
    fn with_pos(self, pos: Pos) -> Self {
        Self { pos, ..self }
    }
    fn with_size(self, size: Size) -> Self {
        Self { size, ..self }
    }
    fn spacer(self, ratio: f32, direction: Direction) -> Self {
        use Direction::*;
        let size = self.size;
        let size = match direction {
            Vertical => Size { height: ratio * size.height, ..size },
            Horizontal => Size { width: ratio * size.width, ..size },
        };
        Self { size, ..self }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Container {
    direction: Direction,
    space_use: SpaceUse,
}
#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Vertical,
    Horizontal,
}
#[derive(Clone, Copy, PartialEq)]
enum SpaceUse {
    Stretch,
    Compact,
}
#[derive(Clone, Copy, PartialEq)]
struct Spacer {
    parent_ratio: f32,
}
#[derive(Component)]
enum Node {
    Container(Container),
    Spacer(Spacer),
    Static(Size),
}

#[derive(Clone, Copy, PartialEq)]
struct AtOutput {
    axe_offset: f32,
    cross_size: f32,
}
// This functions' responsability is to compute the layout for `current` Entity.
//
// Nodes will set **their own size** with the `to_update` query.
// While **the position of the children** will be set with `to_update`.
//
// There is a variety of ways the computation is impossible.
//
// - The tree is invalid according to this modules doc, in which case a warning
//   shall be emitted
// - There isn't enough information to place a node.
fn layout_at(
    current: Entity,
    direction: Direction,
    bounds: PosRect,
    to_update: &mut Query<&mut PosRect>,
    nodes: &Query<(&Node, &Children)>,
) -> AtOutput {
    use SpaceUse::*;
    match nodes.get(current).unwrap() {
        (Node::Container(Container { direction: node_dir, space_use }), children) => {
            if children.is_empty() {
                return AtOutput { axe_offset: 0., cross_size: 0. };
            }
            let mut children_size = 0.0;
            let mut cross_max = 0.0_f32;
            for child in children {
                let result = layout_at(*child, *node_dir, bounds, to_update, nodes);
                children_size += result.axe_offset;
                cross_max = cross_max.max(result.cross_size);
            }
            let size = match space_use {
                Stretch => {
                    let total_space_between = bounds.size.on(*node_dir) - children_size;
                    let space_between = total_space_between / (children.len() - 1) as f32;
                    let mut iter = to_update.iter_many_mut(children);
                    let mut axis_offset = 0.0;
                    while let Some(mut space) = iter.fetch_next() {
                        space.pos.set_axis(*node_dir, axis_offset);
                        // TODO: centering (should be (bounds.size.cross(*node_dir) - space.size.cross(*node_dir)) / 2.0)
                        space.pos.set_cross(*node_dir, 0.0);
                        axis_offset += space.size.on(*node_dir) + space_between;
                    }
                    Size::with(*node_dir, bounds.size.on(*node_dir), cross_max)
                }
                Compact => {
                    let mut axis_offset = 0.0;
                    let mut iter = to_update.iter_many_mut(children);
                    while let Some(mut space) = iter.fetch_next() {
                        space.pos.set_axis(*node_dir, axis_offset);
                        space.pos.set_cross(*node_dir, 0.0);
                        axis_offset += space.size.on(*node_dir);
                    }
                    Size::with(*node_dir, children_size, cross_max)
                }
            };
            let mut to_update = to_update.get_mut(current).unwrap();
            to_update.size = size;
            let (axe_offset, cross_size) = if direction == *node_dir {
                (children_size, cross_max)
            } else {
                (cross_max, children_size)
            };
            AtOutput { axe_offset, cross_size }
        }
        (Node::Spacer(spacer), _) => AtOutput {
            axe_offset: bounds.size.on(direction) * spacer.parent_ratio,
            cross_size: 0.0,
        },
        (Node::Static(size), _) => AtOutput {
            axe_offset: size.on(direction),
            cross_size: size.cross(direction),
        },
    }
}
