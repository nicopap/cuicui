# Bevy UI layout offset

A small bevy plugin to manipulate UI element transform.

Full code:

```rust
use bevy::prelude::*;

#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
pub struct UiOffset(pub Transform);

fn offset(mut query: Query<(&mut Transform, &UiOffset)>) {
    query.for_each_mut(|(mut transform, offset)| {
        *transform = transform.mul_transform(offset.0);
    })
}

pub struct OffsetPlugin;
impl Plugin for OffsetPlugin {
    fn build(&self, app: &mut App) {
        use bevy::transform::TransformSystem;
        use bevy::ui::UiSystem;

        app.add_system(
            offset
                .after(UiSystem::Flex)
                .before(TransformSystem::TransformPropagate),
        );
    }
}
```