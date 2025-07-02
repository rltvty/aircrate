use std::time::Duration;

use tokio::sync::{mpsc, watch};

use crate::{
    channels::{AppState},
};

pub async fn http_streaming_task(
    audio_tx: mpsc::Sender<Vec<u8>>,
    ui_tx: watch::Sender<AppState>,
) {
    println!("📡 HTTP Streaming task started");

    let mut state = ui_tx.borrow().clone();
    let current_channel = state.current_channel.clone().unwrap();
    
    let mut stream_url: Option<String> = None;
    for stream in current_channel.streams.iter() {
        if stream.bitrate == 320 && stream.encoding == "aac" {
            stream_url = Some(stream.url.clone());
            break;
        }
    }
    if stream_url.is_none() {
        println!("❌ Unable to get channel stream");
        return;
    }
    // Update UI to show connecting
    state.stream_status = format!("Connecting to {}...", current_channel.name);
    let _ = ui_tx.send(state);

    loop {
        match connect_and_stream(&stream_url.clone().unwrap(), &audio_tx, &ui_tx).await {
            Ok(_) => {
                println!("🔄 Stream ended normally, reconnecting...");
            }
            Err(e) => {
                eprintln!("❌ Stream error: {}, retrying in 1 seconds...", e);

                // Update UI to show error
                let mut state = ui_tx.borrow().clone();
                state.stream_status = format!("Error: {} (retrying...)", e);
                state.is_streaming = false;
                let _ = ui_tx.send(state);

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn connect_and_stream(
    url: &str,
    audio_tx: &mpsc::Sender<Vec<u8>>,
    ui_tx: &watch::Sender<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use futures_util::StreamExt;

    println!("🌐 Connecting to: {}", url);

    // Create HTTP client with streaming support
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
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

        // Send to audio processing
        match audio_tx.send(chunk.to_vec()).await {
            Ok(_) => {}
            Err(_) => {
                println!("⚠️  Audio channel closed, stopping stream");
                break;
            }
        }

        // Update UI every 50 chunks (roughly every few seconds)
        if chunk_count % 50 == 0 {
            let mut state = ui_tx.borrow().clone();
            state.stream_status = format!(
                "Streaming: {} chunks, {} KB",
                chunk_count,
                total_bytes / 1024
            );
            let _ = ui_tx.send(state);

            println!(
                "📊 Streamed {} chunks, {} KB total",
                chunk_count,
                total_bytes / 1024
            );
        }
    }

    Ok(())
}
