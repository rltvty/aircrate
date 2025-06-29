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
    pub artwork_url: Option<String>,
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
            update_interval: Duration::from_secs(5), // Poll every 5 seconds - be respectful to API
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
        
        // Parse the API response - track info is nested under "trackInfo" object
        let track_info = &track_data["trackInfo"];
        let track = TrackInfo {
            title: track_info["title"].as_str().unwrap_or("Unknown").to_string(),
            artist: track_info["artistCredits"].as_str().unwrap_or("Unknown").to_string(),
            album: track_info["release"].as_str().map(|s| s.to_string()),
            duration: track_info["duration"].as_u64().map(|d| d as u32),
            started_at: track_data["lastUpdated"].as_str().map(|s| s.to_string()),
            artwork_url: track_info["artwork"].as_str().map(|s| s.to_string()),
        };
        
        Ok(track)
    }
}

pub fn update_track_info_system(
    mut track_manager: ResMut<TrackInfoManager>,
    runtime: Res<bevy_tokio_tasks::TokioTasksRuntime>,
    // database: Option<Res<crate::database::Database>>,
) {
    if !track_manager.should_update() {
        return;
    }
    
    // Update timestamp
    track_manager.last_update = Some(std::time::Instant::now());
    
    let api_url = track_manager.api_url.clone();
    let current_track = track_manager.current_track.clone();
    // let db = database.map(|d| (*d).clone());
    
    runtime.spawn_background_task(move |mut ctx| async move {
        match reqwest::get(&api_url).await {
            Ok(response) => {
                match response.json::<serde_json::Value>().await {
                    Ok(track_data) => {
                        let track_info = &track_data["trackInfo"];
                        let new_track = TrackInfo {
                            title: track_info["title"].as_str().unwrap_or("Unknown").to_string(),
                            artist: track_info["artistCredits"].as_str().unwrap_or("Unknown").to_string(),
                            album: track_info["release"].as_str().map(|s| s.to_string()),
                            duration: track_info["duration"].as_u64().map(|d| d as u32),
                            started_at: track_data["lastUpdated"].as_str().map(|s| s.to_string()),
                            artwork_url: track_info["artwork"].as_str().map(|s| s.to_string()),
                        };
                        
                        // Check if track changed
                        let track_changed = match &current_track {
                            Some(current) => current.title != new_track.title || current.artist != new_track.artist,
                            None => true,
                        };
                        
                        if track_changed {
                            println!("Track changed to: {} - {}", new_track.artist, new_track.title);
                            
                            // Store rich metadata in database if available
                            // TODO: Re-enable database storage once SQLite setup is complete
                            if track_data.get("trackInfo")
                                .and_then(|t| t.get("artists"))
                                .map(|a| a.as_array().map(|arr| !arr.is_empty()).unwrap_or(false))
                                .unwrap_or(false) 
                            {
                                println!("Rich metadata detected! (Database storage temporarily disabled)");
                            }
                            
                            // Send track change event back to main thread
                            ctx.run_on_main_thread(move |ctx| {
                                // Get previous track info first
                                let previous_track = {
                                    let track_manager = ctx.world.resource::<TrackInfoManager>();
                                    track_manager.current_track.clone()
                                };
                                
                                // Send event
                                {
                                    let mut track_changed_events = ctx.world.resource_mut::<Events<TrackChangedEvent>>();
                                    track_changed_events.send(TrackChangedEvent {
                                        previous_track,
                                        current_track: new_track.clone(),
                                    });
                                }
                                
                                // Update track manager
                                {
                                    let mut track_manager = ctx.world.resource_mut::<TrackInfoManager>();
                                    track_manager.current_track = Some(new_track);
                                }
                            }).await;
                        } else {
                            // Update current track even if not changed (for timestamp updates)
                            ctx.run_on_main_thread(move |ctx| {
                                let mut track_manager = ctx.world.resource_mut::<TrackInfoManager>();
                                track_manager.current_track = Some(new_track);
                            }).await;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse track info JSON: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch track info: {}", e);
            }
        }
    });
}

pub fn handle_track_changed(
    mut track_changed_events: EventReader<TrackChangedEvent>,
    mut audio_manager: ResMut<crate::audio::AudioStreamManager>,
) {
    for event in track_changed_events.read() {
        println!("Track changed to: {} - {}", event.current_track.artist, event.current_track.title);
        
        // If we have a previous track and we're currently recording, send save command
        if let Some(ref prev_track) = event.previous_track {
            if audio_manager.is_recording {
                if let Some(ref sender) = audio_manager.save_track_sender {
                    if let Err(_) = sender.send((prev_track.artist.clone(), prev_track.title.clone(), prev_track.artwork_url.clone())) {
                        eprintln!("Failed to send save command for track: {} - {}", prev_track.artist, prev_track.title);
                    } else {
                        println!("Sent save command for completed track: {} - {}", prev_track.artist, prev_track.title);
                    }
                }
            }
        }
        
        // Update current recording track
        audio_manager.current_recording_track = Some(event.current_track.clone());
    }
}