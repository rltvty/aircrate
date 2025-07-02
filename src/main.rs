use tokio::sync::{mpsc, watch};

use crate::{
    audio_output::audio_output_task,
    bevy_ui::launch_bevy_app,
    channels::{get_channel, track_info_task, AppState, TrackBoundary},
    http_stream::http_streaming_task,
};

mod audio_output;
mod bevy_ui;
mod channels;
mod http_stream;
mod streaming_reader;

fn main() {
    println!("🎵 AirCrate - Tokio-First Architecture");

    // Create a multi-threaded tokio runtime that doesn't block the main thread
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    let channel = match rt.block_on(get_channel("clubsandwich")) {
        Ok(result) => result,
        Err(e) => {
            println!("❌ Unable to get channel info: {}", e);
            return;
        }
    };

    let mut app_state = AppState::default();
    app_state.current_channel = Some(channel);

    // Create communication channels
    let (audio_tx, audio_rx) = mpsc::channel::<Vec<u8>>(100);
    let (track_tx, track_rx) = mpsc::channel::<TrackBoundary>(10);
    let (ui_tx, ui_rx) = watch::channel::<AppState>(app_state);

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
    launch_bevy_app(ui_rx, rt);
}
