use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Event)]
pub struct StartStreamEvent {
    pub url: String,
}

#[derive(Event)]
pub struct StopStreamEvent;

#[derive(Event)]
pub struct StartAudioEvent;

#[derive(Event)]
pub struct StopAudioEvent;

#[derive(Event)]
pub struct StartRecordingEvent;

#[derive(Event)]
pub struct StopRecordingEvent;

#[derive(Component)]
pub struct StreamState {
    pub url: String,
    pub is_playing: bool,
    pub current_track: Option<String>,
    pub recording_file: Option<PathBuf>,
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            url: String::new(),
            is_playing: false,
            current_track: None,
            recording_file: None,
        }
    }
}

pub fn handle_stream_events(
    mut start_events: EventReader<StartStreamEvent>,
    mut stop_events: EventReader<StopStreamEvent>,
    mut start_audio_events: EventReader<StartAudioEvent>,
    mut stop_audio_events: EventReader<StopAudioEvent>,
    mut start_recording_events: EventReader<StartRecordingEvent>,
    mut stop_recording_events: EventReader<StopRecordingEvent>,
    mut audio_manager: ResMut<crate::audio::AudioStreamManager>,
    runtime: Res<bevy_tokio_tasks::TokioTasksRuntime>,
) {
    for event in start_events.read() {
        println!("Starting stream: {}", event.url);
        audio_manager.start_stream(&event.url, &runtime);
    }
    
    for _event in stop_events.read() {
        println!("Stopping stream");
        audio_manager.stop_stream();
    }
    
    for _event in start_audio_events.read() {
        if audio_manager.is_streaming && !audio_manager.is_playing_audio {
            if let Some(_audio_sender) = audio_manager.audio_sender.clone() {
                let audio_cancel_token = tokio_util::sync::CancellationToken::new();
                audio_manager.audio_cancel_token = Some(audio_cancel_token.clone());
                
                // Just set the playing state - the audio thread is already running
                println!("Starting audio playback");
                audio_manager.is_playing_audio = true;
            }
        }
    }
    
    for _event in stop_audio_events.read() {
        audio_manager.stop_audio_playback();
    }
    
    for _event in start_recording_events.read() {
        audio_manager.start_recording();
    }
    
    for _event in stop_recording_events.read() {
        audio_manager.stop_recording();
    }
}

// Constants for the flux music stream - AAC format (working for playback)
pub const FLUX_STREAM_URL: &str = "https://51-83-105-37-28ffe3.sfn.edge-ovh-gra5.streams.radiosphere.io/557b7263-9216-46b5-a813-a156ffbc9acb/channels/00fc5593-857e-4672-92a5-ac289a98ec01/stream.aac?source=fluxmusic.api.radiosphere.io&quality=10";

// Alternative MP3 stream URL  
pub const FLUX_STREAM_MP3_URL: &str = "https://51-83-105-37-28ffe3.sfn.edge-ovh-gra5.streams.radiosphere.io/557b7263-9216-46b5-a813-a156ffbc9acb/channels/00fc5593-857e-4672-92a5-ac289a98ec01/stream.mp3?source=fluxmusic.api.radiosphere.io&quality=10";

pub const FLUX_TRACK_API: &str = "https://fluxmusic.api.radiosphere.io/channels/00fc5593-857e-4672-92a5-ac289a98ec01/current-track";