pub mod stream;
pub mod track_info;
pub mod http_stream;

pub use stream::*;
pub use track_info::*;

use bevy::prelude::*;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AudioStreamManager::new())
            .add_event::<StartStreamEvent>()
            .add_event::<StopStreamEvent>()
            .add_event::<TrackChangedEvent>()
            .insert_resource(TrackInfoManager::default())
            .add_systems(Startup, setup_audio_system)
            .add_systems(Update, (
                handle_stream_events,
                update_track_info_system,
                handle_track_changed,
            ));
    }
}

#[derive(Resource)]
pub struct AudioStreamManager {
    pub is_playing: bool,
    pub current_url: Option<String>,
    pub cancel_token: Option<tokio_util::sync::CancellationToken>,
}

impl Default for AudioStreamManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioStreamManager {
    pub fn new() -> Self {
        Self {
            is_playing: false,
            current_url: None,
            cancel_token: None,
        }
    }
    
    pub fn start_stream(&mut self, url: &str, recording_path: Option<&str>, runtime: &bevy_tokio_tasks::TokioTasksRuntime) {
        self.stop_stream();
        
        // Create new cancellation token for this stream
        let cancel_token = tokio_util::sync::CancellationToken::new();
        self.cancel_token = Some(cancel_token.clone());
        
        println!("Starting HTTP stream with recording from: {}", url);
        if let Some(path) = recording_path {
            println!("Recording to: {}", path);
        }
        
        // Spawn a task to handle HTTP streaming with recording
        let url_clone = url.to_string();
        let recording_path_clone = recording_path.map(|p| p.to_string());
        
        runtime.spawn_background_task(move |_ctx| async move {
            match crate::audio::http_stream::HttpStreamReader::new(&url_clone).await {
                Ok(reader) => {
                    println!("Successfully connected to HTTP stream");
                    
                    // Create recording file if path provided
                    let mut recording_file = if let Some(path) = recording_path_clone {
                        // Create directory if it doesn't exist
                        if let Some(parent) = std::path::Path::new(&path).parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        match std::fs::File::create(&path) {
                            Ok(file) => {
                                println!("Created recording file: {}", path);
                                Some(file)
                            }
                            Err(e) => {
                                eprintln!("Failed to create recording file {}: {}", path, e);
                                None
                            }
                        }
                    } else {
                        None
                    };
                    
                    // Read chunks and optionally record
                    let mut chunk_count = 0;
                    let mut total_bytes = 0;
                    
                    while reader.is_active().await && !cancel_token.is_cancelled() {
                        if let Some(chunk_result) = reader.read_chunk().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    total_bytes += chunk.len();
                                    chunk_count += 1;
                                    
                                    // Write to recording file if enabled
                                    if let Some(ref mut file) = recording_file {
                                        // For the first chunk, we might want to verify it's a valid audio stream
                                        // For now, just write the raw data - this works better with MP3 streams
                                        if let Err(e) = std::io::Write::write_all(file, &chunk) {
                                            eprintln!("Recording write error: {}", e);
                                        }
                                    }
                                    
                                    // Log progress every 100 chunks
                                    if chunk_count % 100 == 0 {
                                        println!("Streamed {} chunks, {} KB total", chunk_count, total_bytes / 1024);
                                        
                                        // Flush recording file periodically
                                        if let Some(ref mut file) = recording_file {
                                            let _ = std::io::Write::flush(file);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("HTTP stream error: {}", e);
                                    break;
                                }
                            }
                        } else {
                            println!("Stream ended");
                            break;
                        }
                        
                        // Check for cancellation
                        if cancel_token.is_cancelled() {
                            println!("Stream cancelled");
                            break;
                        }
                    }
                    
                    // Final flush and close
                    if let Some(ref mut file) = recording_file {
                        let _ = std::io::Write::flush(file);
                        println!("Recording completed: {} chunks, {} KB total", chunk_count, total_bytes / 1024);
                    }
                    
                    reader.stop().await;
                    println!("HTTP stream with recording completed");
                }
                Err(e) => {
                    eprintln!("Failed to connect to HTTP stream: {}", e);
                }
            }
        });
        
        self.is_playing = true;
        self.current_url = Some(url.to_string());
    }
    
    pub fn stop_stream(&mut self) {
        println!("Stopping stream");
        
        // Cancel the current stream task if running
        if let Some(token) = &self.cancel_token {
            token.cancel();
        }
        
        self.is_playing = false;
        self.current_url = None;
        self.cancel_token = None;
    }
}

fn setup_audio_system() {
    println!("Pure Rust audio system initialized");
}