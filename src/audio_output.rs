use crate::{
    channels::{AppState, TrackBoundary},
    streaming_reader::StreamingReader,
};

use tokio::sync::{mpsc, watch};

pub async fn audio_output_task(
    mut audio_rx: mpsc::Receiver<Vec<u8>>,
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

    loop {
        tokio::select! {
            // Process audio chunks
            Some(chunk) = audio_rx.recv() => {
                if let Err(_) = audio_sender.send(chunk) {
                    println!("⚠️  Audio playback thread disconnected");
                    break;
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
    println!("🎵 Audio playback thread started");

    // Initialize audio output
    let stream_handle = match rodio::OutputStreamBuilder::open_default_stream() {
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
    let sink = rodio::Sink::connect_new(stream_handle.mixer());

    // Create streaming reader that implements Read + Seek
    let streaming_reader = StreamingReader::new(audio_receiver);

    // Sleep a second to let the stream buffer start to fill
    std::thread::sleep(std::time::Duration::from_secs(1));

    loop {
        // Create decoder using Rodio's built-in Symphonia integration
        match rodio::Decoder::builder()
            .with_data(streaming_reader.clone())
            .with_hint("aac")
            .with_gapless(false)
            .with_seekable(false)
            .build()
        {
            Ok(source) => {
                println!("✅ Rodio decoder created successfully");

                // Append the decoded source to the sink
                sink.append(source);

                set_playing_state(&ui_tx, true);

                // Sleep until playback ends (keeps the sink alive)
                println!("🎵 Starting audio playback - sleeping until end");
                sink.sleep_until_end();
                set_playing_state(&ui_tx, false);
            }
            Err(e) => {
                eprintln!(
                    "❌ Failed to create Rodio decoder: {}, retrying in 5 seconds...",
                    e
                );
                 set_playing_state(&ui_tx, false);
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        }
        
    }
}

fn set_playing_state(ui_tx: &watch::Sender<AppState>, is_playing: bool) {
    let mut state = ui_tx.borrow().clone();
    state.is_playing = is_playing;
    let _ = ui_tx.send(state);
}