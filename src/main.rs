use bevy::{color::palettes::css::*, prelude::*, window::{CompositeAlphaMode, MonitorSelection, WindowPosition}};

mod colors;
mod config;
mod window_manager;

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


    let recent_tracks = commands
        .spawn((
            Node {
                width: Val::Px(400.),
                height: Val::Px(100.),
                ..default()
            },
            BorderRadius::all(Val::Px(20.)),
            BackgroundColor(AirCrateColors::dark_blue_ui_panel()),
        ))
        .id();
    
    let border_node = commands
        .spawn((
            Node {
                width: Val::Px(500.),
                height: Val::Px(500.),
                border: UiRect::all(Val::Px(10.)),
                margin: UiRect::all(Val::Px(20.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
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
        .add_child(recent_tracks)
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
    let container = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        })
        .add_children(&[border_node])
        .id();

    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_self: AlignSelf::Stretch,
                justify_self: JustifySelf::Stretch,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                align_content: AlignContent::FlexStart,
                ..default()
            },
            BackgroundColor(AirCrateColors::background_purple()),
        ))
        .add_child(container);
}