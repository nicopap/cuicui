//! A simplified implementation of the classic game "Breakout".

// Derived from bevy's example, https://github.com/bevyengine/bevy
// See LICENSE file in bevy's repo for copyright and distribution

use core::fmt;

use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
    sprite::MaterialMesh2dBundle,
    utils::HashMap,
};
use cuicui_richtext::{track, ResourceTrackerExt, RichTextBundle, RichTextPlugin, WorldBindings};

const TIME_STEP: f32 = 1.0 / 60.0;

const PADDLE_SIZE: Vec3 = Vec3::new(120.0, 20.0, 0.0);
const GAP_BETWEEN_PADDLE_AND_FLOOR: f32 = 60.0;
const PADDLE_SPEED: f32 = 500.0;
const PADDLE_PADDING: f32 = 10.0;

const BALL_STARTING_POSITION: Vec3 = Vec3::new(0.0, -50.0, 1.0);
const BALL_SIZE: Vec3 = Vec3::new(30.0, 30.0, 0.0);
const BALL_SPEED: f32 = 400.0;
const INITIAL_BALL_DIRECTION: Vec2 = Vec2::new(0.5, -0.5);

const WALL_THICKNESS: f32 = 10.0;
// x coordinates
const LEFT_WALL: f32 = -450.;
const RIGHT_WALL: f32 = 450.;
const BOTTOM: f32 = -300.;
const BELLOW_BOTTOM: f32 = 150.;
const TOP_WALL: f32 = 300.;

const BRICK_SIZE: Vec2 = Vec2::new(100., 30.);
// These values are exact
const GAP_BETWEEN_PADDLE_AND_BRICKS: f32 = 270.0;
const GAP_BETWEEN_BRICKS: f32 = 5.0;
// These values are lower bounds, as the number of bricks is computed
const GAP_BETWEEN_BRICKS_AND_CEILING: f32 = 20.0;
const GAP_BETWEEN_BRICKS_AND_SIDES: f32 = 20.0;

const SCOREBOARD_FONT_SIZE: f32 = 40.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

const BACKGROUND_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const PADDLE_COLOR: Color = Color::rgb(0.3, 0.3, 0.7);
const BALL_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);
const BRICK_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const WALL_COLOR: Color = Color::rgb(0.8, 0.8, 0.8);
const TEXT_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RichTextPlugin)
        .init_tracked_resource::<Score>()
        .init_debug_tracked_resource::<Deaths>()
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .init_resource::<FontHandles>()
        .add_startup_system(setup)
        .add_event::<CollisionEvent>()
        // Add our gameplay simulation systems to the fixed timestep schedule
        .add_systems(
            (
                apply_velocity,
                detect_ball_death.after(apply_velocity),
                move_paddle
                    .before(check_for_collisions)
                    .after(apply_velocity),
                check_for_collisions.after(apply_velocity),
                update_ball_bindings.after(apply_velocity),
            )
                .in_schedule(CoreSchedule::FixedUpdate),
        )
        // Configure how frequently our gameplay systems are run
        .insert_resource(FixedTime::new_from_secs(TIME_STEP))
        .add_system(bevy::window::close_on_esc)
        .run();
}

#[derive(Component)]
struct Paddle;

#[derive(Component)]
struct Ball;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component, Debug, Default)]
struct Collider {
    collision_count: usize,
}

#[derive(Default)]
struct CollisionEvent;

#[derive(Component)]
struct Brick;

#[derive(Resource)]
struct CollisionSound(Handle<AudioSource>);

#[derive(Bundle)]
struct WallBundle {
    sprite_bundle: SpriteBundle,
    collider: Collider,
}
enum WallLocation {
    Left,
    Right,
    Top,
}
impl WallLocation {
    fn position(&self) -> Vec2 {
        match self {
            WallLocation::Left => Vec2::new(LEFT_WALL, 0.),
            WallLocation::Right => Vec2::new(RIGHT_WALL, 0.),
            WallLocation::Top => Vec2::new(0., TOP_WALL),
        }
    }

    fn size(&self) -> Vec2 {
        let arena_height = TOP_WALL - BOTTOM;
        let arena_width = RIGHT_WALL - LEFT_WALL;
        // Make sure we haven't messed up our constants
        assert!(arena_height > 0.0);
        assert!(arena_width > 0.0);

        match self {
            WallLocation::Left | WallLocation::Right => {
                Vec2::new(WALL_THICKNESS, arena_height + WALL_THICKNESS)
            }
            WallLocation::Top => Vec2::new(arena_width + WALL_THICKNESS, WALL_THICKNESS),
        }
    }
}

impl WallBundle {
    fn new(location: WallLocation) -> WallBundle {
        WallBundle {
            sprite_bundle: SpriteBundle {
                transform: Transform {
                    translation: location.position().extend(0.0),
                    scale: location.size().extend(1.0),
                    ..default()
                },
                sprite: Sprite { color: WALL_COLOR, ..default() },
                ..default()
            },
            collider: Collider::default(),
        }
    }
}

// This resource tracks the game's score
#[derive(Resource, Default, Reflect)]
struct Score {
    score: usize,
}
impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.score.fmt(f)
    }
}
// This resource tracks how many times the ball fell off the arena.
#[derive(Resource, Default, Debug, Reflect)]
struct Deaths(usize);

#[derive(Resource)]
struct FontHandles(HashMap<&'static str, Handle<Font>>);
impl FromWorld for FontHandles {
    fn from_world(world: &mut World) -> Self {
        let mut map = HashMap::default();
        let server = world.resource::<AssetServer>();
        map.insert(
            "fonts/FiraMono-Medium.ttf",
            server.load("fonts/FiraMono-Medium.ttf"),
        );
        map.insert(
            "fonts/FiraSans-Bold.ttf",
            server.load("fonts/FiraSans-Bold.ttf"),
        );
        Self(map)
    }
}

// Add the game's entities to our world
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2dBundle::default());

    let paddle_y = BOTTOM + GAP_BETWEEN_PADDLE_AND_FLOOR;

    commands.spawn((
        SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, paddle_y, 0.0),
                scale: PADDLE_SIZE,
                ..default()
            },
            sprite: Sprite { color: PADDLE_COLOR, ..default() },
            ..default()
        },
        Paddle,
        track!('d, paddle_hits, Collider::default()),
    ));

    // Ball
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::default().into()).into(),
            material: materials.add(ColorMaterial::from(BALL_COLOR)),
            transform: Transform::from_translation(BALL_STARTING_POSITION).with_scale(BALL_SIZE),
            ..default()
        },
        Ball,
        Velocity(INITIAL_BALL_DIRECTION.normalize() * BALL_SPEED),
    ));

    // Score
    commands.spawn(
        RichTextBundle::parse(
            "Score: {font: fonts/FiraMono-Medium.ttf, color: rgb(1.0, 0.5, 0.5), content: $Score}\n\
            {color: rgb(1.0, 0.2, 0.2), content: $Deaths}\n\
            Paddle hits: {color: pink, content: $paddle_hits}\n\
            Ball position: {font: fonts/FiraMono-Medium.ttf,color: pink|\\{x: {ball_x}, y: {ball_y}\\}}",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: SCOREBOARD_FONT_SIZE,
                    color: TEXT_COLOR,
                },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: SCOREBOARD_TEXT_PADDING,
                left: SCOREBOARD_TEXT_PADDING,
                ..default()
            },
            ..default()
        }),
    );

    // Walls
    commands.spawn(WallBundle::new(WallLocation::Left));
    commands.spawn(WallBundle::new(WallLocation::Right));
    commands.spawn(WallBundle::new(WallLocation::Top));

    let total_width_of_bricks = (RIGHT_WALL - LEFT_WALL) - 2. * GAP_BETWEEN_BRICKS_AND_SIDES;
    let bottom_edge_of_bricks = paddle_y + GAP_BETWEEN_PADDLE_AND_BRICKS;
    let total_height_of_bricks = TOP_WALL - bottom_edge_of_bricks - GAP_BETWEEN_BRICKS_AND_CEILING;

    let n_columns = (total_width_of_bricks / (BRICK_SIZE.x + GAP_BETWEEN_BRICKS)).floor() as usize;
    let n_rows = (total_height_of_bricks / (BRICK_SIZE.y + GAP_BETWEEN_BRICKS)).floor() as usize;
    let n_vertical_gaps = n_columns - 1;

    let center_of_bricks = (LEFT_WALL + RIGHT_WALL) / 2.0;
    let left_edge_of_bricks = center_of_bricks
        - (n_columns as f32 / 2.0 * BRICK_SIZE.x)
        - n_vertical_gaps as f32 / 2.0 * GAP_BETWEEN_BRICKS;

    let offset_x = left_edge_of_bricks + BRICK_SIZE.x / 2.;
    let offset_y = bottom_edge_of_bricks + BRICK_SIZE.y / 2.;

    for row in 0..n_rows {
        for column in 0..n_columns {
            let brick_position = Vec2::new(
                offset_x + column as f32 * (BRICK_SIZE.x + GAP_BETWEEN_BRICKS),
                offset_y + row as f32 * (BRICK_SIZE.y + GAP_BETWEEN_BRICKS),
            );

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite { color: BRICK_COLOR, ..default() },
                    transform: Transform {
                        translation: brick_position.extend(0.0),
                        scale: Vec3::new(BRICK_SIZE.x, BRICK_SIZE.y, 1.0),
                        ..default()
                    },
                    ..default()
                },
                Brick,
                Collider::default(),
            ));
        }
    }
}

fn update_ball_bindings(
    mut context: ResMut<WorldBindings>,
    ball_pos: Query<&Transform, With<Ball>>,
) {
    let Ok(ball_pos) = ball_pos.get_single() else { return; };
    context.set_content("ball_x", &format!("{:.1}", ball_pos.translation.x));
    context.set_content("ball_y", &format!("{:.1}", ball_pos.translation.y));
}
fn detect_ball_death(mut ball_pos: Query<&mut Transform, With<Ball>>, mut deaths: ResMut<Deaths>) {
    let Ok(mut ball_pos) = ball_pos.get_single_mut() else { return; };
    if ball_pos.translation.y < BOTTOM - BELLOW_BOTTOM {
        deaths.0 += 1;
        ball_pos.translation = BALL_STARTING_POSITION;
    }
}
fn move_paddle(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Paddle>>,
) {
    let mut paddle_transform = query.single_mut();
    let mut direction = 0.0;
    if keyboard_input.pressed(KeyCode::Left) {
        direction -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::Right) {
        direction += 1.0;
    }
    let new_paddle_position = paddle_transform.translation.x + direction * PADDLE_SPEED * TIME_STEP;
    let left_bound = LEFT_WALL + WALL_THICKNESS / 2.0 + PADDLE_SIZE.x / 2.0 + PADDLE_PADDING;
    let right_bound = RIGHT_WALL - WALL_THICKNESS / 2.0 - PADDLE_SIZE.x / 2.0 - PADDLE_PADDING;

    paddle_transform.translation.x = new_paddle_position.clamp(left_bound, right_bound);
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * TIME_STEP;
        transform.translation.y += velocity.y * TIME_STEP;
    }
}

fn check_for_collisions(
    mut commands: Commands,
    mut scoreboard: ResMut<Score>,
    mut ball_query: Query<(&mut Velocity, &Transform), With<Ball>>,
    mut collider_query: Query<(Entity, &Transform, &mut Collider, Option<&Brick>)>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    let (mut ball_velocity, ball_transform) = ball_query.single_mut();
    let ball_size = ball_transform.scale.truncate();

    // check collision with walls
    for (collider_entity, transform, mut collider, maybe_brick) in &mut collider_query {
        let collision = collide(
            ball_transform.translation,
            ball_size,
            transform.translation,
            transform.scale.truncate(),
        );
        if let Some(collision) = collision {
            // Sends a collision event so that other systems can react to the collision
            collision_events.send_default();
            collider.collision_count += 1;

            // Bricks should be despawned and increment the scoreboard on collision
            if maybe_brick.is_some() {
                scoreboard.score += 1;
                commands.entity(collider_entity).despawn();
            }
            let mut reflect_x = false;
            let mut reflect_y = false;
            match collision {
                Collision::Left => reflect_x = ball_velocity.x > 0.0,
                Collision::Right => reflect_x = ball_velocity.x < 0.0,
                Collision::Top => reflect_y = ball_velocity.y < 0.0,
                Collision::Bottom => reflect_y = ball_velocity.y > 0.0,
                Collision::Inside => { /* do nothing */ }
            }
            if reflect_x {
                ball_velocity.x = -ball_velocity.x;
            }
            if reflect_y {
                ball_velocity.y = -ball_velocity.y;
            }
        }
    }
}
