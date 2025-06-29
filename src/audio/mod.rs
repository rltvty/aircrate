pub mod stream;
pub mod track_info;
pub mod http_stream;

pub use stream::*;
pub use track_info::*;

use bevy::prelude::*;
use std::sync::mpsc;
use bytes::Bytes;

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
    pub temp_recording_buffer: Vec<u8>,
    pub current_recording_track: Option<crate::audio::TrackInfo>,
    pub save_track_sender: Option<mpsc::Sender<(String, String)>>,
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
            temp_recording_buffer: Vec::new(),
            current_recording_track: None,
            save_track_sender: None,
        }
    }
    
    pub fn start_stream(&mut self, url: &str, recording_path: Option<&str>, runtime: &bevy_tokio_tasks::TokioTasksRuntime) {
        self.stop_stream();
        
        // Create new cancellation token for this stream
        let cancel_token = tokio_util::sync::CancellationToken::new();
        self.cancel_token = Some(cancel_token.clone());
        
        // Create channel for track save commands
        let (save_tx, save_rx) = mpsc::channel::<(String, String)>(); // (artist, title)
        self.save_track_sender = Some(save_tx);
        
        println!("Starting HTTP stream with track-based recording from: {}", url);
        
        // Clear any existing temporary buffer
        self.temp_recording_buffer.clear();
        
        // Spawn a task to handle HTTP streaming with recording
        let url_clone = url.to_string();
        let _recording_path_clone = recording_path.map(|p| p.to_string());
        
        runtime.spawn_background_task(move |_ctx| async move {
            match crate::audio::http_stream::HttpStreamReader::new(&url_clone).await {
                Ok(reader) => {
                    println!("Successfully connected to HTTP stream");
                    
                    // Recording will be handled via temp buffer and track-based saving
                    
                    // Create channel for audio playback only
                    let (audio_tx, audio_rx) = mpsc::channel::<Bytes>();
                    let cancel_token_audio = cancel_token.clone();
                    
                    // Spawn a dedicated thread for audio playback
                    std::thread::spawn(move || {
                        use rodio::{OutputStream, Sink};
                        use std::io::Cursor;
                        use std::collections::VecDeque;
                        
                        // Initialize audio output in the dedicated thread
                        if let Ok((_stream, handle)) = OutputStream::try_default() {
                            println!("Audio output initialized on dedicated thread");
                            
                            // Create a single sink for continuous playback
                            if let Ok(sink) = Sink::try_new(&handle) {
                                // Buffer to accumulate audio data for better continuity
                                let mut audio_buffer = Vec::new();
                                let target_buffer_size = 32768; // 32KB buffer
                                
                                loop {
                                    // Check for cancellation first
                                    if cancel_token_audio.is_cancelled() {
                                        println!("Audio playback stopping due to cancellation");
                                        sink.stop();
                                        break;
                                    }
                                    
                                    // Try to receive audio chunks with timeout
                                    match audio_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                                        Ok(chunk) => {
                                            // Add chunk to buffer
                                            audio_buffer.extend_from_slice(&chunk);
                                            
                                            // When we have enough data, try to decode and play
                                            if audio_buffer.len() >= target_buffer_size {
                                                let cursor = Cursor::new(audio_buffer.clone());
                                                match rodio::Decoder::new(cursor) {
                                                    Ok(source) => {
                                                        sink.append(source);
                                                        // Clear the buffer after successful decode
                                                        audio_buffer.clear();
                                                    }
                                                    Err(_) => {
                                                        // If decode fails, keep accumulating data
                                                        // But prevent buffer from growing too large
                                                        if audio_buffer.len() > target_buffer_size * 4 {
                                                            // Remove first quarter of buffer to make room
                                                            audio_buffer.drain(0..target_buffer_size);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(mpsc::RecvTimeoutError::Timeout) => {
                                            // Timeout is normal, just continue the loop to check cancellation
                                            continue;
                                        }
                                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                                            // Channel disconnected, streaming stopped
                                            println!("Audio channel disconnected, stopping playback");
                                            break;
                                        }
                                    }
                                }
                                
                                // Don't process remaining buffer on stop - we want immediate silence
                                println!("Audio playback thread ending");
                            }
                        } else {
                            eprintln!("Failed to initialize audio output");
                        }
                    });
                    
                    // Create a simple buffer to collect recording data in this task
                    let mut temp_buffer = Vec::new();
                    
                    // Read chunks and optionally record
                    let mut chunk_count = 0;
                    let mut total_bytes = 0;
                    
                    while reader.is_active().await && !cancel_token.is_cancelled() {
                        // Check for save commands (non-blocking)
                        if let Ok((artist, title)) = save_rx.try_recv() {
                            if !temp_buffer.is_empty() {
                                println!("Saving completed track: {} - {} ({} KB)", artist, title, temp_buffer.len() / 1024);
                                
                                // Create filename from track info
                                let safe_artist = sanitize_filename(&artist);
                                let safe_title = sanitize_filename(&title);
                                let filename = format!("./recordings/{} - {}.aac", safe_artist, safe_title);
                                
                                // Create directory if it doesn't exist
                                if let Some(parent) = std::path::Path::new(&filename).parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                
                                // Save the buffer to file
                                match std::fs::write(&filename, &temp_buffer) {
                                    Ok(_) => {
                                        println!("Successfully saved: {}", filename);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to save track {}: {}", filename, e);
                                    }
                                }
                                
                                // Clear the buffer for the new track
                                temp_buffer.clear();
                            }
                        }
                        
                        if let Some(chunk_result) = reader.read_chunk().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    total_bytes += chunk.len();
                                    chunk_count += 1;
                                    
                                    // Send audio chunk to playback thread
                                    if let Err(_) = audio_tx.send(chunk.clone()) {
                                        eprintln!("Audio playback thread disconnected");
                                    }
                                    
                                    // Add chunk to local recording buffer
                                    temp_buffer.extend_from_slice(&chunk);
                                    
                                    // Log progress every 100 chunks
                                    if chunk_count % 100 == 0 {
                                        println!("Streamed {} chunks, {} KB total", chunk_count, total_bytes / 1024);
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
                    
                    println!("Recording completed: {} chunks, {} KB total", chunk_count, total_bytes / 1024);
                    
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
        
        // Clear incomplete recording (as requested - only save complete songs)
        if !self.temp_recording_buffer.is_empty() {
            println!("Discarding incomplete recording ({} KB)", self.temp_recording_buffer.len() / 1024);
            self.temp_recording_buffer.clear();
        }
        
        self.is_playing = false;
        self.current_url = None;
        self.cancel_token = None;
        self.current_recording_track = None;
        self.save_track_sender = None;
    }
}

fn setup_audio_system() {
    println!("Pure Rust audio system initialized");
}

fn sanitize_filename(name: &str) -> String {
    // Replace invalid filename characters with underscores
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}