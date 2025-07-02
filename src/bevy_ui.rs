
use bevy::prelude::*;
use tokio::sync::watch;

use crate::channels::AppState;

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

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Status text
    commands.spawn((
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
    ));

    // Track text
    commands.spawn((
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
    ));
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
