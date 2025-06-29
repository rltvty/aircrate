use bevy::{color::palettes::css::*, prelude::*, window::{CompositeAlphaMode, MonitorSelection, WindowPosition}};

mod colors;
mod config;
mod window_manager;
mod audio;
mod database;

use colors::AirCrateColors;
use config::AppConfig;
use window_manager::{WindowManager, restore_window_position, track_window_changes, save_window_state_on_close};
use audio::{AudioPlugin, StartStreamEvent, StopStreamEvent, StartAudioEvent, StopAudioEvent, StartRecordingEvent, StopRecordingEvent, AudioStreamManager, TrackInfoManager, FLUX_STREAM_URL};
use database::Database;


fn main() {
    let config = AppConfig::load();
    
    // Initialize database
    let rt = tokio::runtime::Runtime::new().unwrap();
    let database = rt.block_on(async {
        Database::new("sqlite:./aircrate.db").await.ok()
    });
    
    let mut app = App::new();
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
        .add_plugins(AudioPlugin)
        .add_plugins(bevy_tokio_tasks::TokioTasksPlugin::default())
        .insert_resource(WindowManager::new())
        .add_systems(Startup, setup)
        .add_systems(Update, (
            restore_window_position, 
            track_window_changes, 
            save_window_state_on_close, 
            handle_ui_buttons,
            update_button_appearance,
            update_track_display
        ))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Stream connection button
    let stream_button = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(150.),
                height: Val::Px(50.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(5.)),
                ..default()
            },
            BackgroundColor(AirCrateColors::highlight_neon_blue()),
            BorderRadius::all(Val::Px(10.)),
            StreamButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Connect"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                StreamButtonText,
            ));
        })
        .id();

    // Audio playback button
    let audio_button = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(150.),
                height: Val::Px(50.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(5.)),
                ..default()
            },
            BackgroundColor(AirCrateColors::thumbs_up_blue()),
            BorderRadius::all(Val::Px(10.)),
            AudioButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Play Audio"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                AudioButtonText,
            ));
        })
        .id();

    // Recording button
    let record_button = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(150.),
                height: Val::Px(50.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(5.)),
                ..default()
            },
            BackgroundColor(AirCrateColors::cassette_orange()),
            BorderRadius::all(Val::Px(10.)),
            RecordButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Record"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                RecordButtonText,
            ));
        })
        .id();

    // Button container
    let button_container = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            margin: UiRect::all(Val::Px(10.)),
            ..default()
        })
        .add_children(&[stream_button, audio_button, record_button])
        .id();

    let current_track_panel = commands
        .spawn((
            Node {
                width: Val::Px(400.),
                height: Val::Px(100.),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(10.)),
                ..default()
            },
            BorderRadius::all(Val::Px(20.)),
            BackgroundColor(AirCrateColors::dark_blue_ui_panel()),
        ))
        .with_children(|parent| {
            // Artist name
            parent.spawn((
                Text::new("Unknown Artist"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(AirCrateColors::primary_text()),
                CurrentArtistText,
            ));
            
            // Track title
            parent.spawn((
                Text::new("Unknown Track"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                CurrentTrackText,
            ));
        })
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
        .add_child(current_track_panel)
        .id();

    let container = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        })
        .add_children(&[button_container, border_node])
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

#[derive(Component)]
struct StreamButton;

#[derive(Component)]
struct StreamButtonText;

#[derive(Component)]
struct AudioButton;

#[derive(Component)]
struct AudioButtonText;

#[derive(Component)]
struct RecordButton;

#[derive(Component)]
struct RecordButtonText;

#[derive(Component)]
struct CurrentArtistText;

#[derive(Component)]
struct CurrentTrackText;

fn handle_ui_buttons(
    mut stream_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<StreamButton>)>,
    mut audio_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<AudioButton>, Without<StreamButton>, Without<RecordButton>)>,
    mut record_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<RecordButton>, Without<StreamButton>, Without<AudioButton>)>,
    mut start_stream_events: EventWriter<StartStreamEvent>,
    mut stop_stream_events: EventWriter<StopStreamEvent>,
    mut start_audio_events: EventWriter<StartAudioEvent>,
    mut stop_audio_events: EventWriter<StopAudioEvent>,
    mut start_recording_events: EventWriter<StartRecordingEvent>,
    mut stop_recording_events: EventWriter<StopRecordingEvent>,
    audio_manager: Res<AudioStreamManager>,
) {
    // Handle Stream button
    for (interaction, mut background_color) in stream_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if audio_manager.is_streaming {
                    println!("Disconnect button pressed!");
                    stop_stream_events.write(StopStreamEvent);
                } else {
                    println!("Connect button pressed!");
                    start_stream_events.write(StartStreamEvent {
                        url: FLUX_STREAM_URL.to_string(),
                    });
                }
            }
            Interaction::Hovered => {
                *background_color = if audio_manager.is_streaming {
                    BackgroundColor(AirCrateColors::thumbs_down_red())
                } else {
                    BackgroundColor(AirCrateColors::thumbs_up_blue())
                };
            }
            Interaction::None => {
                *background_color = if audio_manager.is_streaming {
                    BackgroundColor(AirCrateColors::double_up_pink())
                } else {
                    BackgroundColor(AirCrateColors::highlight_neon_blue())
                };
            }
        }
    }

    // Handle Audio button
    for (interaction, mut background_color) in audio_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if audio_manager.is_playing_audio {
                    println!("Stop audio button pressed!");
                    stop_audio_events.write(StopAudioEvent);
                } else {
                    println!("Start audio button pressed!");
                    start_audio_events.write(StartAudioEvent);
                }
            }
            Interaction::Hovered => {
                *background_color = if audio_manager.is_playing_audio {
                    BackgroundColor(AirCrateColors::thumbs_down_red())
                } else {
                    BackgroundColor(AirCrateColors::thumbs_up_blue())
                };
            }
            Interaction::None => {
                *background_color = if audio_manager.is_playing_audio {
                    BackgroundColor(AirCrateColors::double_up_pink())
                } else {
                    BackgroundColor(AirCrateColors::thumbs_up_blue())
                };
            }
        }
    }

    // Handle Record button
    for (interaction, mut background_color) in record_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if audio_manager.is_recording {
                    println!("Stop recording button pressed!");
                    stop_recording_events.write(StopRecordingEvent);
                } else {
                    println!("Start recording button pressed!");
                    start_recording_events.write(StartRecordingEvent);
                }
            }
            Interaction::Hovered => {
                *background_color = if audio_manager.is_recording {
                    BackgroundColor(AirCrateColors::thumbs_down_red())
                } else {
                    BackgroundColor(AirCrateColors::thumbs_up_blue())
                };
            }
            Interaction::None => {
                *background_color = if audio_manager.is_recording {
                    BackgroundColor(AirCrateColors::double_up_pink())
                } else {
                    BackgroundColor(AirCrateColors::cassette_orange())
                };
            }
        }
    }
}

fn update_track_display(
    track_manager: Res<TrackInfoManager>,
    mut artist_query: Query<&mut Text, (With<CurrentArtistText>, Without<CurrentTrackText>)>,
    mut track_query: Query<&mut Text, (With<CurrentTrackText>, Without<CurrentArtistText>)>,
) {
    if track_manager.is_changed() {
        if let Some(ref track) = track_manager.current_track {
            // Update artist text
            for mut text in artist_query.iter_mut() {
                **text = track.artist.clone();
            }
            
            // Update track text
            for mut text in track_query.iter_mut() {
                **text = track.title.clone();
            }
        }
    }
}

fn update_button_appearance(
    audio_manager: Res<AudioStreamManager>,
    mut stream_text_query: Query<&mut Text, (With<StreamButtonText>, Without<AudioButtonText>, Without<RecordButtonText>)>,
    mut audio_text_query: Query<&mut Text, (With<AudioButtonText>, Without<StreamButtonText>, Without<RecordButtonText>)>,
    mut record_text_query: Query<&mut Text, (With<RecordButtonText>, Without<StreamButtonText>, Without<AudioButtonText>)>,
) {
    if audio_manager.is_changed() {
        // Update stream button text
        for mut text in stream_text_query.iter_mut() {
            **text = if audio_manager.is_streaming {
                "Disconnect".to_string()
            } else {
                "Connect".to_string()
            };
        }
        
        // Update audio button text
        for mut text in audio_text_query.iter_mut() {
            **text = if audio_manager.is_playing_audio {
                "Stop Audio".to_string()
            } else {
                "Play Audio".to_string()
            };
        }
        
        // Update record button text
        for mut text in record_text_query.iter_mut() {
            **text = if audio_manager.is_recording {
                "Stop Record".to_string()
            } else {
                "Record".to_string()
            };
        }
    }
}