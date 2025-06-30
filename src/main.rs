use tokio::sync::{mpsc, watch};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub data: Vec<u8>,
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct TrackBoundary {
    pub artist: String,
    pub title: String,
    pub artwork_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub name: String,
    pub subtitle: String,
    pub stream_aac_hq: String,
    pub stream_aac_lq: String,
    pub stream_mp3_hq: String,
    pub stream_mp3_lq: String,
}

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub is_streaming: bool,
    pub is_playing: bool,
    pub is_recording: bool,
    pub current_track: Option<TrackBoundary>,
    pub stream_status: String,
    pub current_channel: Option<String>,
}

fn main() {
    println!("🎵 AirCrate - Tokio-First Architecture");
    
    // Create a multi-threaded tokio runtime that doesn't block the main thread
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");
    
    // Create communication channels
    let (audio_tx, audio_rx) = mpsc::channel::<AudioFrame>(100);
    let (track_tx, track_rx) = mpsc::channel::<TrackBoundary>(10);
    let (ui_tx, ui_rx) = watch::channel::<AppState>(AppState::default());
    
    // Spawn all our async tasks in the tokio runtime
    let ui_tx_1 = ui_tx.clone();
    rt.spawn(async move {
        http_streaming_task(audio_tx, ui_tx_1).await;
    });
    
    let ui_tx_2 = ui_tx.clone();
    rt.spawn(async move {
        track_info_task(track_tx, ui_tx_2).await;
    });
    
    rt.spawn(async move {
        audio_output_task(audio_rx, track_rx, ui_tx).await;
    });
    
    // Launch Bevy on the main thread (required for macOS)
    println!("🚀 Launching Bevy UI on main thread...");
    ui::launch_bevy_app(ui_rx, rt);
}

async fn http_streaming_task(audio_tx: mpsc::Sender<AudioFrame>, ui_tx: watch::Sender<AppState>) {
    println!("📡 HTTP Streaming task started");
    
    // Get Clubsandwich channel info
    let clubsandwich = get_clubsandwich_channel();
    let stream_url = &clubsandwich.stream_aac_hq; // Use high-quality AAC
    
    // Update UI to show connecting
    let mut state = ui_tx.borrow().clone();
    state.stream_status = format!("Connecting to {}...", clubsandwich.name);
    state.current_channel = Some(clubsandwich.name.clone());
    let _ = ui_tx.send(state);
    
    loop {
        match connect_and_stream(stream_url, &audio_tx, &ui_tx).await {
            Ok(_) => {
                println!("🔄 Stream ended normally, reconnecting...");
            }
            Err(e) => {
                eprintln!("❌ Stream error: {}, retrying in 5 seconds...", e);
                
                // Update UI to show error
                let mut state = ui_tx.borrow().clone();
                state.stream_status = format!("Error: {} (retrying...)", e);
                state.is_streaming = false;
                let _ = ui_tx.send(state);
                
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn connect_and_stream(
    url: &str, 
    audio_tx: &mpsc::Sender<AudioFrame>, 
    ui_tx: &watch::Sender<AppState>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use futures_util::StreamExt;
    
    println!("🌐 Connecting to: {}", url);
    
    // Create HTTP client with streaming support
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    
    // Make the request
    let response = client.get(url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }
    
    println!("✅ Connected! Status: {}", response.status());
    
    // Update UI to show connected
    let mut state = ui_tx.borrow().clone();
    state.is_streaming = true;
    state.stream_status = "Connected - streaming audio".to_string();
    let _ = ui_tx.send(state);
    
    // Get the response body as a stream
    let mut stream = response.bytes_stream();
    let mut chunk_count = 0;
    let mut total_bytes = 0;
    
    // Process chunks as they arrive
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        
        if chunk.is_empty() {
            continue;
        }
        
        chunk_count += 1;
        total_bytes += chunk.len();
        
        // Create audio frame from chunk
        let frame = AudioFrame {
            data: chunk.to_vec(),
            timestamp: std::time::Instant::now(),
        };
        
        // Send to audio processing
        if audio_tx.send(frame).await.is_err() {
            println!("⚠️  Audio channel closed, stopping stream");
            break;
        }
        
        // Update UI every 50 chunks (roughly every few seconds)
        if chunk_count % 50 == 0 {
            let mut state = ui_tx.borrow().clone();
            state.stream_status = format!("Streaming: {} chunks, {} KB", chunk_count, total_bytes / 1024);
            let _ = ui_tx.send(state);
            
            println!("📊 Streamed {} chunks, {} KB total", chunk_count, total_bytes / 1024);
        }
    }
    
    Ok(())
}

fn get_clubsandwich_channel() -> Channel {
    Channel {
        name: "Clubsandwich".to_string(),
        subtitle: "24/7 Electronic Music Selected by FluxFM".to_string(),
        stream_aac_hq: "https://fluxmusic.api.radiosphere.io/channels/clubsandwich/stream.aac?quality=10".to_string(),
        stream_aac_lq: "https://fluxmusic.api.radiosphere.io/channels/clubsandwich/stream.aac?quality=1".to_string(),
        stream_mp3_hq: "https://fluxmusic.api.radiosphere.io/channels/clubsandwich/stream.mp3?quality=10".to_string(),
        stream_mp3_lq: "https://fluxmusic.api.radiosphere.io/channels/clubsandwich/stream.mp3?quality=1".to_string(),
    }
}

// Future: Add async function to fetch all channels from API
#[allow(dead_code)]
async fn fetch_available_channels() -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get("https://fluxmusic.api.radiosphere.io/channels").await?;
    let channels_data: serde_json::Value = response.json().await?;
    
    let mut channels = Vec::new();
    
    if let Some(channels_array) = channels_data.as_array() {
        for channel_data in channels_array {
            if let (Some(name), Some(subtitle)) = (
                channel_data["name"].as_str(),
                channel_data["subtitle"].as_str(),
            ) {
                let base_url = format!("https://fluxmusic.api.radiosphere.io/channels/{}/stream", name.to_lowercase());
                channels.push(Channel {
                    name: name.to_string(),
                    subtitle: subtitle.to_string(),
                    stream_aac_hq: format!("{}.aac?quality=10", base_url),
                    stream_aac_lq: format!("{}.aac?quality=1", base_url),
                    stream_mp3_hq: format!("{}.mp3?quality=10", base_url),
                    stream_mp3_lq: format!("{}.mp3?quality=1", base_url),
                });
            }
        }
    }
    
    Ok(channels)
}

async fn get_channel_id(channel_name: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    println!("🔍 Looking up channel ID for: {}", channel_name);
    
    // Fetch the channels list
    let response = reqwest::get("https://fluxmusic.api.radiosphere.io/channels").await?;
    
    if !response.status().is_success() {
        return Err(format!("Channels API error: {}", response.status()).into());
    }
    
    let channels_data: serde_json::Value = response.json().await?;
    
    // The API returns an object with a "data" field containing the channels array
    if let Some(channels_array) = channels_data["data"].as_array() {
        println!("🔍 Found {} channels in API response", channels_array.len());
        for (index, channel) in channels_array.iter().enumerate() {
            if let (Some(name), Some(id)) = (
                channel["name"].as_str(),
                channel["channelId"].as_str(), // Note: it's "channelId", not "id"
            ) {
                println!("  ✓ Channel {}: '{}' (ID: {})", index, name, id);
                if name.to_lowercase() == channel_name.to_lowercase() {
                    println!("✅ Found matching channel '{}' with ID: {}", name, id);
                    return Ok(id.to_string());
                }
            } else {
                println!("  ❌ Channel {} missing name or channelId field", index);
            }
        }
        println!("❌ No channel found matching '{}'", channel_name);
    } else {
        println!("❌ API response missing 'data' array field");
    }
    
    Err(format!("Channel '{}' not found in API response", channel_name).into())
}

async fn track_info_task(track_tx: mpsc::Sender<TrackBoundary>, ui_tx: watch::Sender<AppState>) {
    println!("🎵 Track info task started");
    
    // First, get the Clubsandwich channel ID dynamically
    let channel_id = match get_channel_id("clubsandwich").await {
        Ok(id) => {
            println!("✅ Found Clubsandwich channel ID: {}", id);
            id
        }
        Err(e) => {
            eprintln!("❌ Failed to get channel ID: {}", e);
            println!("⚠️  Falling back to hardcoded ID");
            "00fc5593-857e-4672-92a5-ac289a98ec01".to_string() // Fallback to known ID
        }
    };
    
    let api_url = format!("https://fluxmusic.api.radiosphere.io/channels/{}/current-track", channel_id);
    println!("🔗 Track API URL: {}", api_url);
    
    let mut interval = tokio::time::interval(Duration::from_secs(5)); // Poll every 5 seconds
    let mut last_track_id: Option<String> = None;
    
    loop {
        interval.tick().await;
        
        match fetch_current_track(&api_url).await {
            Ok(track_info) => {
                // Check if this is a new track
                let track_id = format!("{}_{}", track_info.artist, track_info.title);
                
                if last_track_id.as_ref() != Some(&track_id) {
                    println!("🎵 Track changed: {} - {}", track_info.artist, track_info.title);
                    
                    // Send track boundary for recording system
                    if track_tx.send(track_info.clone()).await.is_err() {
                        println!("⚠️  Track channel closed, stopping track poller");
                        break;
                    }
                    
                    // Update UI state
                    let mut state = ui_tx.borrow().clone();
                    state.current_track = Some(track_info);
                    let _ = ui_tx.send(state);
                    
                    last_track_id = Some(track_id);
                } else {
                    // Same track, just update timestamp if needed
                    println!("🔄 Same track playing: {} - {}", track_info.artist, track_info.title);
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to fetch track info: {}", e);
                
                // Don't spam errors, but keep trying
                if interval.period().as_secs() < 30 {
                    // Increase polling interval on errors to be respectful
                    interval = tokio::time::interval(Duration::from_secs(30));
                    println!("⏳ Increased polling interval due to API errors");
                }
            }
        }
    }
}

async fn fetch_current_track(api_url: &str) -> Result<TrackBoundary, Box<dyn std::error::Error + Send + Sync>> {
    println!("🌐 Fetching track info from: {}", api_url);
    
    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    
    // Make the API request
    let response = client.get(api_url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()).into());
    }
    
    // Parse JSON response
    let track_data: serde_json::Value = response.json().await?;
    
    // Extract track information from the API response
    let track_info = &track_data["trackInfo"];
    
    let artist = track_info["artistCredits"]
        .as_str()
        .unwrap_or("Unknown Artist")
        .to_string();
        
    let title = track_info["title"]
        .as_str()
        .unwrap_or("Unknown Track")
        .to_string();
    
    // Get artwork URL and enhance it if it's from the Flux CDN
    let artwork_url = track_info["artwork"]
        .as_str()
        .map(|url| {
            if url.contains("fluxmusic.cdn.radiosphere.io") {
                format!("{}?type=large", url) // Get high quality artwork
            } else {
                url.to_string()
            }
        });
    
    println!("✅ Fetched track: {} - {} (artwork: {})", 
        artist, 
        title,
        artwork_url.as_ref().map(|_| "yes").unwrap_or("no")
    );
    
    Ok(TrackBoundary {
        artist,
        title,
        artwork_url,
    })
}

async fn audio_output_task(
    mut audio_rx: mpsc::Receiver<AudioFrame>,
    mut track_rx: mpsc::Receiver<TrackBoundary>,
    ui_tx: watch::Sender<AppState>,
) {
    println!("🔊 Audio output task started");
    
    // Spawn the audio playback in a separate thread (rodio requirement)
    let (audio_sender, audio_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
    let ui_tx_audio = ui_tx.clone();
    
    std::thread::spawn(move || {
        audio_playback_thread(audio_receiver, ui_tx_audio);
    });
    
    let mut current_track: Option<TrackBoundary> = None;
    let mut audio_buffer = Vec::new();
    let mut frame_count = 0;
    
    loop {
        tokio::select! {
            // Process audio frames
            Some(frame) = audio_rx.recv() => {
                frame_count += 1;
                
                // Accumulate audio data for decoding
                audio_buffer.extend_from_slice(&frame.data);
                
                // Try to send chunks to audio thread when we have enough data
                if audio_buffer.len() >= 8192 { // 8KB chunks
                    let chunk = audio_buffer.drain(0..8192).collect::<Vec<u8>>();
                    if let Err(_) = audio_sender.send(chunk) {
                        println!("⚠️  Audio playback thread disconnected");
                        break;
                    }
                }
                
                // Update UI periodically
                if frame_count % 100 == 0 {
                    let mut state = ui_tx.borrow().clone();
                    state.is_playing = true;
                    let _ = ui_tx.send(state);
                }
            }
            
            // Handle track boundaries (for recording)
            Some(boundary) = track_rx.recv() => {
                if let Some(prev_track) = current_track.take() {
                    println!("💾 Would save track: {} - {}", prev_track.artist, prev_track.title);
                }
                current_track = Some(boundary);
            }
            
            else => {
                println!("📻 All channels closed, stopping audio output");
                break;
            }
        }
    }
}

fn audio_playback_thread(
    audio_receiver: std::sync::mpsc::Receiver<Vec<u8>>,
    ui_tx: watch::Sender<AppState>,
) {
    use rodio::{OutputStream, Sink, Source};
    use std::io::Cursor;
    
    println!("🎵 Audio playback thread started");
    
    // Initialize audio output
    let (_stream, handle) = match OutputStream::try_default() {
        Ok(output) => {
            println!("✅ Audio output initialized");
            output
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize audio output: {}", e);
            return;
        }
    };
    
    // Create sink for playback
    let sink = match Sink::try_new(&handle) {
        Ok(sink) => {
            println!("✅ Audio sink created");
            sink
        }
        Err(e) => {
            eprintln!("❌ Failed to create audio sink: {}", e);
            return;
        }
    };
    
    let mut audio_buffer = Vec::new();
    let mut chunk_count = 0;
    let target_buffer_size = 32768; // 32KB buffer for better AAC decoding
    
    // Main audio processing loop
    loop {
        match audio_receiver.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(chunk) => {
                chunk_count += 1;
                audio_buffer.extend_from_slice(&chunk);
                
                // Try to decode when we have enough data
                while audio_buffer.len() >= target_buffer_size {
                    let decode_chunk = audio_buffer.drain(0..target_buffer_size).collect::<Vec<u8>>();
                    
                    // Try to decode the chunk with symphonia
                    match decode_audio_chunk(&decode_chunk) {
                        Ok(Some(source)) => {
                            // Keep the sink fed but not overstuffed
                            if sink.len() < 3 {
                                sink.append(source);
                                println!("🔊 Audio decoded and queued! Sink queue length: {}", sink.len());
                                
                                // Update UI to show we're playing
                                if chunk_count % 20 == 0 {
                                    let mut state = ui_tx.borrow().clone();
                                    state.is_playing = true;
                                    let _ = ui_tx.send(state);
                                }
                            } else {
                                println!("⏸️  Sink queue full ({}), skipping chunk", sink.len());
                            }
                        }
                        Ok(None) => {
                            // Not enough data for a complete frame, put some back
                            if decode_chunk.len() > 1024 {
                                audio_buffer.splice(0..0, decode_chunk[1024..].iter().cloned());
                            }
                        }
                        Err(e) => {
                            if chunk_count % 50 == 0 {
                                println!("🔄 Audio decode attempt {} ({})", chunk_count, e);
                            }
                            // Continue trying with more data
                        }
                    }
                }
                
                if chunk_count % 100 == 0 {
                    println!("🎵 Audio: {} chunks processed, {} bytes buffered", chunk_count, audio_buffer.len());
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Normal timeout, continue
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                println!("📻 Audio channel disconnected, stopping playback");
                break;
            }
        }
    }
    
    println!("🔇 Audio playback thread ending");
}

fn decode_audio_chunk(chunk: &[u8]) -> Result<Option<rodio::Decoder<std::io::Cursor<Vec<u8>>>>, Box<dyn std::error::Error>> {
    // Try to decode with rodio (which uses symphonia internally)
    let cursor = std::io::Cursor::new(chunk.to_vec());
    match rodio::Decoder::new(cursor) {
        Ok(decoder) => Ok(Some(decoder)),
        Err(e) => {
            // This is expected for incomplete AAC frames
            Err(format!("Decode error: {}", e).into())
        }
    }
}

mod ui {
    use super::*;
    use bevy::prelude::*;
    use tokio::sync::watch;
    
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
                    state.is_streaming,
                    state.is_playing,
                    state.is_recording,
                    state.stream_status
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
}