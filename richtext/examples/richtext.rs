//! Demonstrates rich text and how to update it.

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_fab::BevyModify;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use cuicui_richtext::{
    modifiers, trait_extensions::*, MakeRichText, Modifier, RichText, RichTextPlugin,
};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(bevy::log::LogPlugin {
                level: bevy::log::Level::DEBUG,
                filter: "\
                    bevy_app=info,\
                    bevy_ecs=info,\
                    cuicui_fab=trace,\
                    cuicui_richtext::modifiers=debug,\
                    cuicui_richtext=trace,\
                    gilrs_core=info,\
                    gilrs=info,\
                    naga=info,\
                    wgpu=warn\
                "
                .to_string(),
            }),
        )
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(RichTextPlugin::new())
        .add_plugin(WorldInspectorPlugin::default())
        .with_formatter("print_interaction", |i: &Interaction, entry| {
            let text = match i {
                Interaction::Clicked => {
                    println!("Clicked!");
                    "Clicked"
                }
                Interaction::Hovered => "Hovered",
                Interaction::None => "None",
            };
            entry
                .modify(|e| e.set_content(format_args!("{text}")))
                .or_insert_with(|| Modifier::init_content(format_args!("{text}")));
        })
        .init_resource::<Fps>()
        .insert_resource(ClearColor(Color::BLACK))
        .register_type::<Fps>()
        .add_startup_system(setup)
        .add_system(fps_update)
        .add_system(greet_update)
        .add_system(color_update)
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

    // Button
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size { width: Val::Percent(100.0), ..default() },
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(ButtonBundle {
                    style: Style {
                        size: Size { width: Val::Px(150.0), height: Val::Px(65.0) },
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: Color::FUCHSIA.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Button",
                        TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 40.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                        },
                    ));
                });
        });

    // Rich Text
    commands.spawn((
        MakeRichText::new(
            "{Color:{color}|{Rainbow:20.0|Bonjour} {greeted}!\n\
            {Color:Yellow, Sine:80|We are having fun here, woopy!}}",
        )
        .with_text_style(TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 100.0,
            color: Color::WHITE,
        })
        .with_text_alignment(TextAlignment::Left)
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
        MakeRichText::new(
            "FPS: {Font:fonts/FiraMono-Medium.ttf, Color:gold, Content:{Res.Fps.fps:.1}}\n\
             Button state: {Marked(Button).Interaction:print_interaction}",
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
fn color_update(time: Res<Time>, mut query: Query<RichText, With<ColorText>>) {
    for mut text in &mut query {
        let seconds = time.elapsed_seconds();
        let new_color = Color::Rgba {
            red: (1.25 * seconds).sin() / 2.0 + 0.5,
            green: (0.75 * seconds).sin() / 2.0 + 0.5,
            blue: (0.50 * seconds).sin() / 2.0 + 0.5,
            alpha: 1.0,
        };
        text.set("color", modifiers::Modifier::color(new_color));
    }
}
fn greet_update(
    time: Res<Time>,
    mut query: Query<RichText, With<ColorText>>,
    mut current_guest: Local<usize>,
) {
    let delta = time.delta_seconds_f64();
    let current_time = time.elapsed_seconds_f64();
    let at_interval = |t: f64| current_time % t < delta;
    for mut text in &mut query {
        if at_interval(1.3) {
            *current_guest = (*current_guest + 1) % GUESTS.len();
            text.set_content("greeted", &GUESTS[*current_guest]);
        }
    }
}

fn fps_update(diagnostics: Res<Diagnostics>, mut fps: ResMut<Fps>, time: Res<Time>) {
    let delta = time.delta_seconds_f64();
    let current_time = time.elapsed_seconds_f64();
    let at_interval = |t: f64| current_time % t < delta;
    if let Some(diag) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(value) = diag.smoothed() {
            if at_interval(0.5) {
                fps.fps = value;
            }
        }
    }
}
