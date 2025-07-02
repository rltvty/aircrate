use crate::{
    channels::{AppState, TrackBoundary},
    streaming_reader::StreamingReader,
};

use tokio::sync::{mpsc, watch};

// the delay (in seconds) between when the new track starts and we get the info about it.
const NEW_TRACK_DELAY: usize = 10;
 
// the amount of extra seconds we want to include before and after the actual track boundries.
const OVERLAP_SECONDS: usize = 20;



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
    let mut track_buffer = Vec::new();
    let mut file_buffer = Vec::new();
    let mut first_track_seen = false;
    let overlap_bytes = seconds_to_bytes(OVERLAP_SECONDS);
    let delay_bytes = seconds_to_bytes(NEW_TRACK_DELAY);


    loop {
        tokio::select! {
            // Process audio chunks
            Some(chunk) = audio_rx.recv() => {
                track_buffer.extend_from_slice(&chunk);

                if let Err(_) = audio_sender.send(chunk) {
                    println!("⚠️  Audio playback thread disconnected");
                    break;
                }
            }

            // Handle track boundaries (for recording)
            Some(boundary) = track_rx.recv() => {
                if let Some(prev_track) = current_track.take() {
                    
                    if first_track_seen {
                        if track_buffer.len() > delay_bytes {
                            let track_boundry = track_buffer.len() - delay_bytes;
                            file_buffer.splice(.., track_buffer[0..track_boundry].iter().cloned());
                            println!("Saving completed track: {} - {} ({} KB)", prev_track.artist, prev_track.title, file_buffer.len() / 1024);
                            
                            // Create filename from track info
                            let safe_artist = sanitize_filename(&prev_track.artist);
                            let safe_title = sanitize_filename(&prev_track.title);
                            let filename = format!("./recordings/{} - {}.aac", safe_artist, safe_title);
                            
                            // Create directory if it doesn't exist
                            if let Some(parent) = std::path::Path::new(&filename).parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            
                            // Save the buffer to file
                            match std::fs::write(&filename, &file_buffer) {
                                Ok(_) => {
                                    println!("Successfully saved: {}", filename);
                                }
                                Err(e) => {
                                    eprintln!("Failed to save track {}: {}", filename, e);
                                }
                            }
                            
                            // Clear the buffer for the new trackAdd commentMore actions
                            file_buffer.clear();
                        }
                    } else {
                        let mut state = ui_tx.borrow().clone();
                        state.is_recording = true;
                        let _ = ui_tx.send(state);
                        first_track_seen = true
                    }
                    
                    if track_buffer.len() > delay_bytes + overlap_bytes {
                        let dump_size = track_buffer.len() - delay_bytes - overlap_bytes;
                        println!("💩 Dumping old track data: {} - {} with length: {} KB", prev_track.artist, prev_track.title, dump_size / 1024);
                        track_buffer.drain(0..dump_size).collect::<Vec<u8>>().iter().for_each(drop);
                    }
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

fn seconds_to_bytes(seconds: usize) -> usize {
    // 320 kbps stream
    // data says we see about 14 KB/s
    seconds * 14 * 1024
}

fn set_playing_state(ui_tx: &watch::Sender<AppState>, is_playing: bool) {
    let mut state = ui_tx.borrow().clone();
    state.is_playing = is_playing;
    let _ = ui_tx.send(state);
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