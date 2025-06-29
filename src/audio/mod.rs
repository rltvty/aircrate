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
            .add_event::<StartAudioEvent>()
            .add_event::<StopAudioEvent>()
            .add_event::<StartRecordingEvent>()
            .add_event::<StopRecordingEvent>()
            .add_event::<TrackChangedEvent>()
            .insert_resource(TrackInfoManager::default())
            .add_systems(Startup, setup_audio_system)
            .add_systems(Update, handle_stream_events)
            .add_systems(Update, update_track_info_system)
            .add_systems(Update, handle_track_changed);
    }
}

#[derive(Resource)]
pub struct AudioStreamManager {
    pub is_streaming: bool, // Whether we're connected to the stream
    pub is_playing_audio: bool, // Whether audio output is enabled
    pub is_recording: bool, // Whether we're recording to disk
    pub current_url: Option<String>,
    pub cancel_token: Option<tokio_util::sync::CancellationToken>,
    pub audio_cancel_token: Option<tokio_util::sync::CancellationToken>, // Separate token for audio
    pub temp_recording_buffer: Vec<u8>,
    pub current_recording_track: Option<crate::audio::TrackInfo>,
    pub save_track_sender: Option<mpsc::Sender<(String, String, Option<String>)>>,
    pub audio_sender: Option<mpsc::Sender<Bytes>>, // Channel to control audio playback
    pub track_end_offset_seconds: u32, // How many seconds to cut off the end for boundary detection
    pub overlap_before_seconds: u32,   // How many seconds of previous track to include
    pub overlap_after_seconds: u32,    // How many seconds of next track to include
}

impl Default for AudioStreamManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioStreamManager {
    pub fn new() -> Self {
        Self {
            is_streaming: false,
            is_playing_audio: false,
            is_recording: false,
            current_url: None,
            cancel_token: None,
            audio_cancel_token: None,
            temp_recording_buffer: Vec::new(),
            current_recording_track: None,
            save_track_sender: None,
            audio_sender: None,
            track_end_offset_seconds: 20, // Default: cut 20 seconds off the end for boundary detection
            overlap_before_seconds: 10,   // Include 10 seconds of previous track
            overlap_after_seconds: 10,    // Include 10 seconds of next track
        }
    }
    
    pub fn start_stream(&mut self, url: &str, runtime: &bevy_tokio_tasks::TokioTasksRuntime) {
        if self.is_streaming {
            return; // Already streaming
        }
        
        self.stop_stream();
        
        // Create new cancellation token for this stream
        let cancel_token = tokio_util::sync::CancellationToken::new();
        self.cancel_token = Some(cancel_token.clone());
        
        // Create channel for track save commands
        let (save_tx, save_rx) = mpsc::channel::<(String, String, Option<String>)>(); // (artist, title, artwork_url)
        self.save_track_sender = Some(save_tx);
        
        // Create channel for audio data
        let (audio_tx, audio_rx) = mpsc::channel::<Bytes>();
        self.audio_sender = Some(audio_tx.clone());
        
        println!("Starting HTTP stream from: {}", url);
        
        // Clear any existing temporary buffer
        self.temp_recording_buffer.clear();
        
        // Spawn audio playback thread separately
        let audio_cancel_token = tokio_util::sync::CancellationToken::new();
        self.audio_cancel_token = Some(audio_cancel_token.clone());
        self.start_audio_playback(audio_rx, audio_cancel_token);
        
        // Spawn a task to handle HTTP streaming
        let url_clone = url.to_string();
        
        runtime.spawn_background_task(move |_ctx| async move {
            match crate::audio::http_stream::HttpStreamReader::new(&url_clone).await {
                Ok(reader) => {
                    println!("Successfully connected to HTTP stream");
                    
                    // Recording will be handled via temp buffer and track-based saving
                    
                    // Audio will be handled by the separate audio playback thread
                    
                    // Create buffers for sophisticated overlap management
                    let mut temp_buffer = Vec::new();
                    let mut previous_track_overlap = Vec::new(); // Overlap from end of previous track
                    
                    // Calculate buffer sizes for overlap management
                    let estimated_bitrate = 128000; // 128 kbps AAC stream
                    let bytes_per_second = estimated_bitrate / 8;
                    let overlap_before_bytes = (10 * bytes_per_second) as usize; // 10 seconds before
                    let overlap_after_bytes = (10 * bytes_per_second) as usize;  // 10 seconds after
                    let _stream_start_time = std::time::Instant::now();
                    
                    // Read chunks and optionally record
                    let mut chunk_count = 0;
                    let mut total_bytes = 0;
                    
                    while reader.is_active().await && !cancel_token.is_cancelled() {
                        // Check for save commands (non-blocking) - only if recording is enabled
                        if let Ok((artist, title, artwork_url)) = save_rx.try_recv() {
                            if !temp_buffer.is_empty() {
                                // Calculate track boundaries with DJ-friendly overlaps
                                let track_end_offset_seconds = 20; // Cut 20 seconds off for boundary detection
                                let bytes_to_trim = (track_end_offset_seconds as usize) * bytes_per_second;
                                
                                // Determine the core track data (without the detection offset)
                                let core_track_size = if temp_buffer.len() > bytes_to_trim {
                                    temp_buffer.len() - bytes_to_trim
                                } else {
                                    temp_buffer.len() / 2 // Fallback if calculation is off
                                };
                                
                                // Build the DJ-friendly track with overlaps
                                let mut dj_track_data = Vec::new();
                                
                                // 1. Add overlap from previous track (10 seconds)
                                dj_track_data.extend_from_slice(&previous_track_overlap);
                                
                                // 2. Add the core track data
                                dj_track_data.extend_from_slice(&temp_buffer[..core_track_size]);
                                
                                // 3. Add overlap for next track (10 seconds after core track)
                                let overlap_start = core_track_size;
                                let overlap_end = std::cmp::min(temp_buffer.len(), overlap_start + overlap_after_bytes);
                                if overlap_start < temp_buffer.len() {
                                    dj_track_data.extend_from_slice(&temp_buffer[overlap_start..overlap_end]);
                                }
                                
                                // Save overlap from end of current track for next track's beginning
                                previous_track_overlap.clear();
                                let overlap_for_next_start = if core_track_size >= overlap_before_bytes {
                                    core_track_size - overlap_before_bytes
                                } else {
                                    0
                                };
                                previous_track_overlap.extend_from_slice(&temp_buffer[overlap_for_next_start..core_track_size]);
                                
                                println!("Saving DJ track: {} - {} ({} KB total, {} KB overlap before, {} KB core, {} KB overlap after)", 
                                    artist, title, 
                                    dj_track_data.len() / 1024,
                                    previous_track_overlap.len() / 1024,
                                    core_track_size / 1024,
                                    (overlap_end - overlap_start) / 1024);
                                
                                // Create filename from track info
                                let safe_artist = sanitize_filename(&artist);
                                let safe_title = sanitize_filename(&title);
                                let filename = format!("./recordings/{} - {}.aac", safe_artist, safe_title);
                                
                                // Create directory if it doesn't exist
                                if let Some(parent) = std::path::Path::new(&filename).parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                
                                // Save the DJ-friendly track data to file
                                match std::fs::write(&filename, &dj_track_data) {
                                    Ok(_) => {
                                        println!("Successfully saved DJ track: {}", filename);
                                        
                                        // Add ID3 tags to the saved file with artwork if available
                                        if let Err(e) = add_id3_tags(&filename, &artist, &title, artwork_url.as_deref()).await {
                                            eprintln!("Failed to add ID3 tags to {}: {}", filename, e);
                                        } else {
                                            println!("Added ID3 tags: {} - {}", artist, title);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to save DJ track {}: {}", filename, e);
                                    }
                                }
                                
                                // Clear the current buffer for the new track (keeping overlap)
                                temp_buffer.clear();
                            }
                        }
                        
                        if let Some(chunk_result) = reader.read_chunk().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    total_bytes += chunk.len();
                                    chunk_count += 1;
                                    
                                    // Send audio chunk to playback thread (always - playback thread will handle enable/disable)
                                    if let Err(_) = audio_tx.send(chunk.clone()) {
                                        // This is expected if audio playback is disabled
                                    }
                                    
                                    // Add chunk to recording buffer (this will be handled by recording state in track change handler)
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
        
        self.is_streaming = true;
        self.current_url = Some(url.to_string());
    }
    
    pub fn start_audio_playback(&mut self, audio_rx: mpsc::Receiver<Bytes>, cancel_token: tokio_util::sync::CancellationToken) {
        if self.is_playing_audio {
            return; // Already playing
        }
        
        self.is_playing_audio = true;
        
        // Spawn dedicated audio playback thread
        std::thread::spawn(move || {
            use rodio::{OutputStream, Sink};
            use std::io::Cursor;
            
            // Initialize audio output in the dedicated thread
            if let Ok((_stream, handle)) = OutputStream::try_default() {
                println!("Audio output initialized on dedicated thread");
                
                // Create a single sink for continuous playback
                if let Ok(sink) = Sink::try_new(&handle) {
                    // Buffer to accumulate audio data for better continuity
                    let mut audio_buffer = Vec::new();
                    let target_buffer_size = 131072; // 128KB buffer for smoother playback
                    
                    loop {
                        // Check for cancellation first
                        if cancel_token.is_cancelled() {
                            println!("Audio playback stopping due to cancellation");
                            sink.stop();
                            break;
                        }
                        
                        // Try to receive audio chunks with timeout
                        match audio_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                            Ok(chunk) => {
                                // Add chunk to buffer
                                audio_buffer.extend_from_slice(&chunk);
                                
                                // Try to decode when we have enough data
                                while audio_buffer.len() >= target_buffer_size {
                                    // Try to decode a portion of the buffer
                                    let decode_chunk_size = target_buffer_size;
                                    let decode_data = audio_buffer.drain(0..decode_chunk_size).collect::<Vec<u8>>();
                                    
                                    let cursor = Cursor::new(decode_data);
                                    match rodio::Decoder::new(cursor) {
                                        Ok(source) => {
                                            // Keep the sink queue well-fed but not overstuffed
                                            if sink.len() < 3 {
                                                sink.append(source);
                                            }
                                        }
                                        Err(_) => {
                                            // If decode fails, this might be a partial frame
                                            // Put some data back and try with more data next time
                                            if audio_buffer.is_empty() {
                                                // Only break if we can't decode anything
                                                break;
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
                    
                    println!("Audio playback thread ending");
                }
            } else {
                eprintln!("Failed to initialize audio output");
            }
        });
    }
    
    pub fn stop_audio_playback(&mut self) {
        if let Some(token) = &self.audio_cancel_token {
            token.cancel();
        }
        self.is_playing_audio = false;
        self.audio_cancel_token = None;
    }
    
    pub fn start_recording(&mut self) {
        self.is_recording = true;
        println!("Recording started");
    }
    
    pub fn stop_recording(&mut self) {
        self.is_recording = false;
        // Clear incomplete recording buffer as requested
        if !self.temp_recording_buffer.is_empty() {
            println!("Discarding incomplete recording ({} KB)", self.temp_recording_buffer.len() / 1024);
            self.temp_recording_buffer.clear();
        }
        println!("Recording stopped");
    }
    
    pub fn stop_stream(&mut self) {
        println!("Stopping stream");
        
        // Cancel the current stream task if running
        if let Some(token) = &self.cancel_token {
            token.cancel();
        }
        
        // Stop audio playback
        self.stop_audio_playback();
        
        // Stop recording
        self.stop_recording();
        
        self.is_streaming = false;
        self.current_url = None;
        self.cancel_token = None;
        self.current_recording_track = None;
        self.save_track_sender = None;
        self.audio_sender = None;
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

async fn download_artwork_for_id3(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Enhance URL to get medium size for ID3 tags (good balance of quality vs file size)
    let enhanced_url = if url.contains("fluxmusic.cdn.radiosphere.io") {
        format!("{}?type=medium", url)
    } else {
        url.to_string()
    };
    
    println!("Downloading artwork for ID3: {}", enhanced_url);
    let response = reqwest::get(&enhanced_url).await?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

async fn add_id3_tags(filename: &str, artist: &str, title: &str, artwork_url: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    use id3::TagLike;
    use chrono::Datelike;
    
    // Note: ID3 tags work best with MP3 files, but we can try with AAC
    // Some players may not read ID3 tags from AAC files properly
    let mut tag = id3::Tag::new();
    
    // Set basic metadata
    tag.set_artist(artist);
    tag.set_title(title);
    tag.set_album("AirCrate Stream Recording"); // Optional: set a default album name
    
    // Set date as a timestamp
    let now = chrono::Utc::now();
    let timestamp = id3::Timestamp {
        year: now.year(),
        month: Some(now.month() as u8),
        day: Some(now.day() as u8),
        hour: None,
        minute: None,
        second: None,
    };
    tag.set_date_recorded(timestamp);
    
    // Add artwork if URL is provided
    if let Some(artwork_url) = artwork_url {
        match download_artwork_for_id3(artwork_url).await {
            Ok(artwork_data) => {
                let picture = id3::frame::Picture {
                    mime_type: "image/jpeg".to_string(),
                    picture_type: id3::frame::PictureType::CoverFront,
                    description: "Album Cover".to_string(),
                    data: artwork_data,
                };
                tag.add_frame(picture);
                println!("Added artwork to ID3 tag");
            }
            Err(e) => {
                eprintln!("Failed to download artwork for ID3: {}", e);
            }
        }
    }
    
    // Write the tag to the file
    match tag.write_to_path(filename, id3::Version::Id3v24) {
        Ok(_) => Ok(()),
        Err(e) => {
            // If direct writing fails, try creating a temporary file approach
            eprintln!("Direct ID3 writing failed: {}, trying alternative approach", e);
            
            // For AAC files, ID3 tags might not be fully supported
            // We could consider converting to MP3 or using a different metadata format
            // For now, we'll just log the issue and continue
            Err(Box::new(e))
        }
    }
}