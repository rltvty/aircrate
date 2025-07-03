
use bevy::prelude::*;
use tokio::sync::watch;
// use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

use crate::{channels::AppState, colors::AirCrateColors};


pub fn launch_bevy_app(ui_rx: watch::Receiver<AppState>, _rt: tokio::runtime::Runtime) {
    // Run Bevy directly on the main thread (required for macOS)
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: "AirCrate - Tokio Edition".into(),
            resolution: (800.0, 600.0).into(),
            ..default()
        }),
        ..default()};

    App::new()
        .add_plugins(DefaultPlugins.set(window_plugin))
        // .add_plugins(EguiPlugin { enable_multipass_for_primary_context: true })
        // .add_plugins(WorldInspectorPlugin::new())
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

    let status_text = (
        Text::new("Initializing..."),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            ..default()
        },
        StatusText,
    );

    let track_text = (
        Text::new("No track playing"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            ..default()
        },
        TrackText,
    );

    let image = asset_server.load("upper_frame.png");

    let slicer = TextureSlicer {
        border: BorderRect {
            left: 61.,
            right: 829.,
            top: 61.,
            bottom: 481.,
        },
        ..default()
    };

    let upper_frame = (
        ImageNode {
            image: image.clone(),
            image_mode: NodeImageMode::Sliced(slicer.clone()),
            ..default()
        },
        Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            // // horizontally center child text
            // justify_content: JustifyContent::Center,
            // // vertically center child text
            align_items: AlignItems::Center,
            //margin: UiRect::all(Val::Px(20.0)),
            ..default()
        }
    );

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
        .with_children(|parent| {
            parent.spawn(upper_frame).with_children(|parent| {
                parent.spawn(status_text);
                parent.spawn(track_text);
            });
        });

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
