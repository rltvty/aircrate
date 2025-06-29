use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<u32>,
    pub started_at: Option<String>,
}

#[derive(Event)]
pub struct TrackChangedEvent {
    pub previous_track: Option<TrackInfo>,
    pub current_track: TrackInfo,
}

#[derive(Resource)]
pub struct TrackInfoManager {
    pub current_track: Option<TrackInfo>,
    pub last_update: Option<std::time::Instant>,
    pub update_interval: Duration,
    pub api_url: String,
}

impl Default for TrackInfoManager {
    fn default() -> Self {
        Self {
            current_track: None,
            last_update: None,
            update_interval: Duration::from_secs(10), // Poll every 10 seconds
            api_url: crate::audio::stream::FLUX_TRACK_API.to_string(),
        }
    }
}

impl TrackInfoManager {
    pub fn should_update(&self) -> bool {
        match self.last_update {
            None => true,
            Some(last) => last.elapsed() >= self.update_interval,
        }
    }
    
    pub async fn fetch_current_track(&self) -> Result<TrackInfo, reqwest::Error> {
        let response = reqwest::get(&self.api_url).await?;
        let track_data: serde_json::Value = response.json().await?;
        
        // Parse the API response - you may need to adjust this based on the actual API structure
        let track = TrackInfo {
            title: track_data["title"].as_str().unwrap_or("Unknown").to_string(),
            artist: track_data["artist"].as_str().unwrap_or("Unknown").to_string(),
            album: track_data["album"].as_str().map(|s| s.to_string()),
            duration: track_data["duration"].as_u64().map(|d| d as u32),
            started_at: track_data["started_at"].as_str().map(|s| s.to_string()),
        };
        
        Ok(track)
    }
}

pub fn update_track_info_system(
    mut track_manager: ResMut<TrackInfoManager>,
    mut track_changed_events: EventWriter<TrackChangedEvent>,
) {
    if !track_manager.should_update() {
        return;
    }
    
    // Update timestamp
    track_manager.last_update = Some(std::time::Instant::now());
    
    // Note: This is a simplified version. In a real implementation, you'd want to
    // use async tasks or a background thread to avoid blocking the main thread
    println!("Would fetch track info from: {}", track_manager.api_url);
    
    // For now, simulate a track change
    let mock_track = TrackInfo {
        title: "Sample Track".to_string(),
        artist: "Sample Artist".to_string(),
        album: Some("Sample Album".to_string()),
        duration: Some(240),
        started_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    
    if let Some(ref current) = track_manager.current_track {
        // Check if track changed (in real implementation, compare with fetched data)
        if current.title != mock_track.title {
            track_changed_events.write(TrackChangedEvent {
                previous_track: Some(current.clone()),
                current_track: mock_track.clone(),
            });
        }
    } else {
        // First track
        track_changed_events.write(TrackChangedEvent {
            previous_track: None,
            current_track: mock_track.clone(),
        });
    }
    
    track_manager.current_track = Some(mock_track);
}

pub fn handle_track_changed(
    mut track_changed_events: EventReader<TrackChangedEvent>,
) {
    for event in track_changed_events.read() {
        println!("Track changed to: {} - {}", event.current_track.artist, event.current_track.title);
        
        if let Some(ref prev) = event.previous_track {
            println!("Previous track was: {} - {}", prev.artist, prev.title);
        }
        
        // TODO: Handle track changes for file splitting
        // - Close current recording file
        // - Start new recording file with track info in filename
    }
}