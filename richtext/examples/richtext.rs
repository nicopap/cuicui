//! Demonstrates rich text and how to update it.

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    ecs::system::EntityCommands,
    prelude::*,
    window::PrimaryWindow,
};
use bevy_fab::BevyModify;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use cuicui_richtext::{
    modifiers, trait_extensions::*, Entry, MakeRichText, Modifier, ReflectQueryable, RichText,
    RichTextPlugin,
};

const MED: f32 = 40.0;
const BIG: f32 = 60.0;
const SML: f32 = 30.0;

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
        .add_sys_fmt(
            "show_bottom",
            |value: &dyn Reflect, entry: Entry<_>, cursor: Query<&Window, With<PrimaryWindow>>| {
                let Some(value) = value.downcast_ref::<Interaction>() else {
                    panic!("We expected an Interaction");
                };
                let cursor = cursor.single();
                println!("{:?}", cursor.physical_cursor_position());
                println!("Bottom interaction: {value:?}");
                *entry.or_insert(Modifier::color(default())) = Modifier::color(match value {
                    Interaction::Clicked => Color::PINK,
                    Interaction::Hovered => Color::GREEN,
                    Interaction::None => Color::BLUE,
                });
            },
        )
        .add_fn_fmt("show_top", |i: &Interaction, entry| {
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
        .register_type::<TopButton>()
        .register_type::<BottomButton>()
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

#[derive(Component, Default, Reflect)]
#[reflect(Component, Queryable)]
struct TopButton;

#[derive(Component, Default, Reflect)]
#[reflect(Component, Queryable)]
struct BottomButton;

fn button<'a, 'w, 's>(
    font: Handle<Font>,
    text: &'static str,
    cmds: &'a mut ChildBuilder<'w, 's, '_>,
) -> EntityCommands<'w, 's, 'a> {
    let color = Color::rgb(0.9, 0.9, 0.9);
    let button_text = TextBundle::from_section(text, TextStyle { font, font_size: SML, color });
    let button_style = ButtonBundle {
        style: Style {
            size: Size { width: Val::Px(250.0), height: Val::Px(65.0) },
            justify_content: JustifyContent::Center,
            margin: UiRect::all(Val::Px(30.0)),
            align_items: AlignItems::Center,
            ..default()
        },
        background_color: Color::FUCHSIA.into(),
        ..default()
    };
    let mut cmds = cmds.spawn(button_style);
    cmds.with_children(|p| {
        p.spawn(button_text);
    });
    cmds
}
fn fancy_text(font: Handle<Font>, mut cmds: EntityCommands) {
    cmds.insert((
        MakeRichText::new(
            "{Color:{color}|{Rainbow:20.0|Bonjour} {greeted}!\n\
            {Color:Yellow, Sine:80|We are having fun here, woopy!}}",
        )
        .with_text_style(TextStyle { font, font_size: BIG, color: Color::WHITE })
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
        Name::new("Fancy text"),
    ));
}
fn button_reactors(font: Handle<Font>, mut cmds: EntityCommands) {
    cmds.insert((
        MakeRichText::new(
            "Top button: {Marked(TopButton).Interaction:show_top}\n\
            {Color: {Marked(BottomButton).Interaction:show_bottom}|Bottom Button state}",
        )
        .with_text_style(TextStyle { font, font_size: MED, color: Color::WHITE }),
        Name::new("Reactors"),
    ));
}
fn fps_text(font: Handle<Font>, mut cmds: EntityCommands) {
    cmds.insert((
        // To use a specific font, you need to hold a handle on it.
        // This is why we added the `FiraMediumHolder` resource earlier,
        // otherwise, the font doesn't show up.
        MakeRichText::new(
            "FPS: {Font:fonts/FiraMono-Medium.ttf, Color:gold, Content:{Res(Fps).fps:.1}}",
        )
        .with_text_style(TextStyle { font, font_size: MED, color: Color::WHITE }),
        Name::new("FPS"),
    ));
}
fn break_text(font: Handle<Font>, mut cmds: EntityCommands) {
    cmds.insert((
        MakeRichText::new("This is \n a test")
            .with_text_style(TextStyle { font, font_size: MED, ..default() })
            .with_text_alignment(TextAlignment::Right),
        Name::new("Linebreak test"),
    ));
}
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2dBundle::default());

    // Standard font
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    // Mono font
    #[derive(Resource)]
    struct FiraMediumHolder(Handle<Font>);
    commands.insert_resource(FiraMediumHolder(
        asset_server.load("fonts/FiraMono-Medium.ttf"),
    ));
    //--------layout----------
    // FPS   |       |    test
    //       | top B |    text
    //       |       |
    // Bonjo | bot B | B text1
    // wavy  |       | B color
    //------------------------

    let direction = |direction, percent| NodeBundle {
        style: Style {
            size: Size::all(Val::Percent(percent)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            flex_direction: direction,
            ..default()
        },
        ..default()
    };
    let vertical = |percent| direction(FlexDirection::Column, percent);
    let horizontal = |percent| direction(FlexDirection::Row, percent);

    commands.spawn(horizontal(100.0)).with_children(|cmds| {
        cmds.spawn(vertical(100.0)).with_children(|cmds| {
            fps_text(font.clone(), cmds.spawn_empty());
            fancy_text(font.clone(), cmds.spawn_empty());
        });
        cmds.spawn(vertical(50.0)).with_children(|cmds| {
            button(font.clone(), "Top Button", cmds).insert(TopButton);
            button(font.clone(), "Bottom Button", cmds).insert(BottomButton);
        });
        cmds.spawn(vertical(100.0)).with_children(|cmds| {
            break_text(font.clone(), cmds.spawn_empty());
            button_reactors(font.clone(), cmds.spawn_empty());
        });
    });
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
