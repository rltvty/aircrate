use bevy::{color::palettes::css::*, prelude::*, window::{CompositeAlphaMode, MonitorSelection, WindowPosition}};

mod colors;
mod config;
mod window_manager;

use bevy_cobweb_ui::ui_bevy::FlexGrow;
use colors::AirCrateColors;
use config::AppConfig;
use window_manager::{WindowManager, restore_window_position, track_window_changes, save_window_state_on_close};


fn main() {
    let config = AppConfig::load();
    
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "AirCrate".into(),
                position: if let Some(monitor_idx) = config.window.monitor_index {
                    println!("boop");
                    WindowPosition::Centered(MonitorSelection::Index(monitor_idx))
                } else {
                    WindowPosition::Centered(MonitorSelection::Primary)
                },
                resolution: (config.window.width, config.window.height).into(),
                resizable: true,
                titlebar_shown: true,
                titlebar_transparent: true,
                has_shadow: false,
                transparent: true,
                composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
                movable_by_window_background: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WindowManager::new())
        .add_systems(Startup, setup)
        .add_systems(Update, (restore_window_position, track_window_changes, save_window_state_on_close))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let current_stream = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                margin: UiRect::all(Val::Px(20.)),
                width: Val::Auto,
                ..default()
            },
            BorderRadius::all(Val::Px(20.)),
            BackgroundColor(AirCrateColors::dark_blue_ui_panel()),
        ))
        .id();

    let recent_tracks = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                margin: UiRect::all(Val::Px(20.)),
                width: Val::Auto,
                ..default()
            },
            BorderRadius::all(Val::Px(20.)),
            BackgroundColor(AirCrateColors::dark_blue_ui_panel()),
        ))
        .id();
    
    let border_node = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(10.)),
                margin: UiRect::all(Val::Px(20.)),
                flex_grow: 1.0,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Stretch,

                ..default()
            },
            BackgroundColor(MAROON.into()),
            BorderColor(RED.into()),
            BorderRadius::all(Val::Px(10.)),
            Outline {
                width: Val::Px(6.),
                offset: Val::Px(6.),
                color: AirCrateColors::border_lines(),
            },
        ))
        .add_children(&[recent_tracks, current_stream])
        .id();
    // let label_node = commands
    //     .spawn((
    //         Text::new("Air Crate"),
    //         TextFont {
    //             font_size: 9.0,
    //             ..Default::default()
    //         },
    //     ))
    //     .id();
    // let container = commands
    //     .spawn(Node {
    //         flex_direction: FlexDirection::Column,
    //         align_items: AlignItems::Center,
    //         ..default()
    //     })
    //     .add_children(&[border_node])
    //     .id();

    let background_container = (
            Node {
                flex_direction: FlexDirection::Column,
                flex_wrap: FlexWrap::NoWrap,
                align_self: AlignSelf::Stretch,
                justify_self: JustifySelf::Stretch,
                ..default()
            },
            BackgroundColor(AirCrateColors::background_purple()),
        );

    commands
        .spawn(background_container)
        .add_child(border_node);
}