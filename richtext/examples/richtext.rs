//! Demonstrates rich text and how to update it.

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use cuicui_richtext::{modifiers, MakeRichTextBundle, RichTextData, RichTextPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
            level: bevy::log::Level::DEBUG,
            filter: "wgpu=warn,bevy_ecs=info,naga=info,bevy_app=info".to_string(),
        }))
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(RichTextPlugin)
        .add_plugin(WorldInspectorPlugin::default())
        .init_resource::<Fps>()
        .register_type::<Fps>()
        .add_startup_system(setup)
        .add_system(text_update_system)
        .add_system(text_color_system)
        .run();
}

// A unit struct to help identify the FPS UI component, since there may be many Text components
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct Fps {
    fps: f64,
}

// A unit struct to help identify the color-changing Text component
#[derive(Component)]
struct ColorText;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        MakeRichTextBundle::new("{Color:{color}|hello\n{greeted}!}")
            .with_text_style(TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 100.0,
                color: Color::WHITE,
            })
            .with_text_alignment(TextAlignment::Center)
            .with_style(Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    bottom: Val::Px(5.0),
                    right: Val::Px(15.0),
                    ..default()
                },
                ..default()
            }),
        ColorText,
        Name::new("Greet"),
    ));

    #[derive(Resource)]
    struct FiraMediumHolder(Handle<Font>);
    commands.insert_resource(FiraMediumHolder(
        asset_server.load("fonts/FiraMono-Medium.ttf"),
    ));
    commands.spawn((
        // To use a specific font, you need to hold a handle on it.
        // This is why we added the `FiraMediumHolder` resource earlier,
        // otherwise, the font doesn't show up.
        MakeRichTextBundle::new(
            "FPS: {Font:fonts/FiraMono-Medium.ttf, Color:gold, Content:{Res.Fps.fps:.1}}",
        )
        .with_text_style(TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 60.0,
            color: Color::WHITE,
        }),
        Name::new("Gold"),
    ));
}

const GUESTS: &[&str] = &["bevy", "boovy", "noovy", "groovy", "bavy", "cuicui"];
fn text_color_system(
    time: Res<Time>,
    mut query: Query<&mut RichTextData, With<ColorText>>,
    mut current_guest: Local<usize>,
) {
    let delta = time.delta_seconds_f64();
    let current_time = time.elapsed_seconds_f64();
    let at_interval = |t: f64| current_time % t < delta;
    for mut text in &mut query {
        let seconds = time.elapsed_seconds();
        let new_color = Color::Rgba {
            red: (1.25 * seconds).sin() / 2.0 + 0.5,
            green: (0.75 * seconds).sin() / 2.0 + 0.5,
            blue: (0.50 * seconds).sin() / 2.0 + 0.5,
            alpha: 1.0,
        };
        text.set("color", new_color);
        if at_interval(1.3) {
            *current_guest = (*current_guest + 1) % GUESTS.len();
            let new_content = modifiers::Content::from(&GUESTS[*current_guest]);
            text.set("greeted", new_content);
        }
    }
}

fn text_update_system(diagnostics: Res<Diagnostics>, mut fps: ResMut<Fps>) {
    if let Some(diag) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(value) = diag.smoothed() {
            fps.fps = value;
        }
    }
}
