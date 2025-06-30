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

async fn track_info_task(track_tx: mpsc::Sender<TrackBoundary>, ui_tx: watch::Sender<AppState>) {
    println!("🎵 Track info task started");
    
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    let tracks = vec![
        ("Daft Punk", "One More Time"),
        ("Justice", "D.A.N.C.E."),
        ("Moderat", "Reminder"),
        ("Woo York", "The Red Room"),
    ];
    let mut track_index = 0;
    
    loop {
        interval.tick().await;
        
        let (artist, title) = tracks[track_index % tracks.len()];
        let boundary = TrackBoundary {
            artist: artist.to_string(),
            title: title.to_string(),
            artwork_url: None,
        };
        
        println!("🎵 Track changed: {} - {}", artist, title);
        
        if track_tx.send(boundary.clone()).await.is_err() {
            println!("⚠️  Track channel closed, stopping track poller");
            break;
        }
        
        // Update UI state
        let mut state = ui_tx.borrow().clone();
        state.current_track = Some(boundary);
        let _ = ui_tx.send(state);
        
        track_index += 1;
    }
}

async fn audio_output_task(
    mut audio_rx: mpsc::Receiver<AudioFrame>,
    mut track_rx: mpsc::Receiver<TrackBoundary>,
    ui_tx: watch::Sender<AppState>,
) {
    println!("🔊 Audio output task started");
    
    let mut frame_count = 0;
    let mut current_track: Option<TrackBoundary> = None;
    
    loop {
        tokio::select! {
            // Process audio frames
            Some(frame) = audio_rx.recv() => {
                frame_count += 1;
                
                // Simulate audio processing
                if frame_count % 200 == 0 {
                    println!("🔊 Processed {} audio frames", frame_count);
                    
                    // Update UI state
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