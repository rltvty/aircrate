use bevy::{color::palettes::css::*, prelude::*};

mod colors;
use colors::AirCrateColors;


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
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