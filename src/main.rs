use bevy::{color::palettes::css::*, prelude::*, window::{CompositeAlphaMode, MonitorSelection, WindowPosition}};

mod colors;
mod config;
mod window_manager;
mod audio;

use colors::AirCrateColors;
use config::AppConfig;
use window_manager::{WindowManager, restore_window_position, track_window_changes, save_window_state_on_close};
use audio::{AudioPlugin, StartStreamEvent, StopStreamEvent, AudioStreamManager, FLUX_STREAM_URL};


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
        .add_plugins(AudioPlugin)
        .add_plugins(bevy_tokio_tasks::TokioTasksPlugin::default())
        .insert_resource(WindowManager::new())
        .add_systems(Startup, setup)
        .add_systems(Update, (
            restore_window_position, 
            track_window_changes, 
            save_window_state_on_close, 
            handle_ui_buttons,
            update_button_appearance
        ))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Play button
    let play_button = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(200.),
                height: Val::Px(50.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(10.)),
                ..default()
            },
            BackgroundColor(AirCrateColors::highlight_neon_blue()),
            BorderRadius::all(Val::Px(10.)),
            PlayButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Play Stream"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                PlayButtonText,
            ));
        })
        .id();

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

    let container = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        })
        .add_children(&[play_button, border_node])
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
struct PlayButton;

#[derive(Component)]
struct PlayButtonText;

fn handle_ui_buttons(
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<PlayButton>)>,
    mut start_stream_events: EventWriter<StartStreamEvent>,
    mut stop_stream_events: EventWriter<StopStreamEvent>,
    audio_manager: Res<AudioStreamManager>,
) {
    for (interaction, mut background_color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if audio_manager.is_playing {
                    println!("Stop button pressed!");
                    stop_stream_events.write(StopStreamEvent);
                } else {
                    println!("Play button pressed!");
                    start_stream_events.write(StartStreamEvent {
                        url: FLUX_STREAM_URL.to_string(),
                        recording_directory: Some(std::path::PathBuf::from("./recordings")),
                    });
                }
            }
            Interaction::Hovered => {
                *background_color = if audio_manager.is_playing {
                    BackgroundColor(AirCrateColors::thumbs_down_red())
                } else {
                    BackgroundColor(AirCrateColors::thumbs_up_blue())
                };
            }
            Interaction::None => {
                *background_color = if audio_manager.is_playing {
                    BackgroundColor(AirCrateColors::double_up_pink())
                } else {
                    BackgroundColor(AirCrateColors::highlight_neon_blue())
                };
            }
        }
    }
}

fn update_button_appearance(
    audio_manager: Res<AudioStreamManager>,
    mut button_query: Query<&mut BackgroundColor, (With<PlayButton>, Without<Interaction>)>,
    mut text_query: Query<&mut Text, With<PlayButtonText>>,
) {
    if audio_manager.is_changed() {
        // Update button background color
        for mut background_color in button_query.iter_mut() {
            *background_color = if audio_manager.is_playing {
                BackgroundColor(AirCrateColors::double_up_pink())
            } else {
                BackgroundColor(AirCrateColors::highlight_neon_blue())
            };
        }
        
        // Update button text
        for mut text in text_query.iter_mut() {
            **text = if audio_manager.is_playing {
                "Stop Recording".to_string()
            } else {
                "Play Stream".to_string()
            };
        }
    }
}