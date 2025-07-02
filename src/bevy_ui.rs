
use bevy::prelude::*;
use tokio::sync::watch;

use crate::{channels::AppState, colors::AirCrateColors};

use bevy::asset::LoadState;

fn check_asset_load_state(
    asset_server: Res<AssetServer>,
    my_handle: Handle<Image>,
) {
    match asset_server.get_load_state(my_handle.id()) {
        Some(LoadState::Loaded) => println!("Asset loaded"),
        Some(LoadState::NotLoaded) => println!("Asset not loaded"),
        Some(LoadState::Loading) => println!("Still loading..."),
        _ => println!("Unknown state"),
    }
}

pub fn launch_bevy_app(ui_rx: watch::Receiver<AppState>, _rt: tokio::runtime::Runtime) {
    // Run Bevy directly on the main thread (required for macOS)
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "AirCrate - Tokio Edition".into(),
                resolution: (800.0, 600.0).into(),
                ..default()
            }),
            ..default()
        }).set( AssetPlugin {
            ..default()
        }))

        .insert_resource(StateReceiver(ui_rx))
        .add_systems(Startup, setup_ui)
        .add_systems(Update, update_ui_from_state)
        .run();
}

#[derive(Resource)]
struct StateReceiver(watch::Receiver<AppState>);

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct TrackText;

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    
    let status_text = commands
        .spawn((
        Text::new("Initializing..."),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(50.0),
            left: Val::Px(50.0),
            ..default()
        },
        StatusText,
    )).id();

    let track_text = commands
        .spawn((
        Text::new("No track playing"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(100.0),
            left: Val::Px(50.0),
            ..default()
        },
        TrackText,
    )).id();

    let image = asset_server.load("upper_frame.png");

    check_asset_load_state(asset_server, image.clone());

    
    let slicer = TextureSlicer {
        border: BorderRect {
            left: 61.,
            right: 829.,
            top: 61.,
            bottom: 481.,
        },
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    };
    
    let border_node = commands.spawn((
        ImageNode {
            image: image.clone(),
            //image_mode: NodeImageMode::Sliced(slicer.clone()),
            ..default()
        },
        Node {
            // horizontally center child text
            justify_content: JustifyContent::Center,
            // vertically center child text
            align_items: AlignItems::Center,
            margin: UiRect::all(Val::Px(20.0)),
            ..default()
        }
    )).add_children(&[status_text, track_text]).id();

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

fn update_ui_from_state(
    mut state_receiver: ResMut<StateReceiver>,
    mut status_query: Query<&mut Text, (With<StatusText>, Without<TrackText>)>,
    mut track_query: Query<&mut Text, (With<TrackText>, Without<StatusText>)>,
) {
    // Check for state updates (non-blocking)
    if state_receiver.0.has_changed().unwrap_or(false) {
        let state = state_receiver.0.borrow_and_update().clone();

        // Update status text
        for mut text in status_query.iter_mut() {
            **text = format!(
                "Streaming: {} | Playing: {} | Recording: {} | {}",
                state.is_streaming, state.is_playing, state.is_recording, state.stream_status
            );
        }

        // Update track text
        for mut text in track_query.iter_mut() {
            **text = if let Some(ref track) = state.current_track {
                format!("🎵 {} - {}", track.artist, track.title)
            } else {
                "No track playing".to_string()
            };
        }
    }
}
