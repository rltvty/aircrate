use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Event)]
pub struct StartStreamEvent {
    pub url: String,
    pub recording_directory: Option<PathBuf>,
}

#[derive(Event)]
pub struct StopStreamEvent;

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
    mut audio_manager: ResMut<crate::audio::AudioStreamManager>,
    runtime: Res<bevy_tokio_tasks::TokioTasksRuntime>,
) {
    for event in start_events.read() {
        println!("Starting stream: {}", event.url);
        
        let recording_path = event.recording_directory.as_ref().map(|dir| {
            // Determine file extension based on URL for proper playback
            let extension = if event.url.contains("stream.mp3") { "mp3" } else { "aac" };
            let filename = format!("aircrate_recording_{}.{}", 
                chrono::Utc::now().format("%Y%m%d_%H%M%S"), extension);
            dir.join(filename).to_string_lossy().to_string()
        });
        
        audio_manager.start_stream(&event.url, recording_path.as_deref(), &runtime);
    }
    
    for _event in stop_events.read() {
        println!("Stopping stream");
        audio_manager.stop_stream();
    }
}

// Constants for the flux music stream - AAC format (working for playback)
pub const FLUX_STREAM_URL: &str = "https://51-83-105-37-28ffe3.sfn.edge-ovh-gra5.streams.radiosphere.io/557b7263-9216-46b5-a813-a156ffbc9acb/channels/00fc5593-857e-4672-92a5-ac289a98ec01/stream.aac?source=fluxmusic.api.radiosphere.io&quality=10";

// Alternative MP3 stream URL  
pub const FLUX_STREAM_MP3_URL: &str = "https://51-83-105-37-28ffe3.sfn.edge-ovh-gra5.streams.radiosphere.io/557b7263-9216-46b5-a813-a156ffbc9acb/channels/00fc5593-857e-4672-92a5-ac289a98ec01/stream.mp3?source=fluxmusic.api.radiosphere.io&quality=10";

pub const FLUX_TRACK_API: &str = "https://fluxmusic.api.radiosphere.io/channels/00fc5593-857e-4672-92a5-ac289a98ec01/current-track";