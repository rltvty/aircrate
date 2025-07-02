use serde::Deserialize;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

#[derive(Debug, Deserialize)]
pub struct ChannelResponse {
    pub items: Vec<ChannelInfo>,
    pub next: Option<String>,
    pub size: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChannelInfo {
    #[serde(rename = "channelId")]
    pub channel_id: String,

    #[serde(rename = "coverImage")]
    pub cover_image: Option<String>,

    #[serde(rename = "displayName")]
    pub display_name: String,

    pub name: String,
    pub streams: Vec<StreamInfo>,

    #[serde(rename = "subTitle")]
    pub sub_title: Option<String>,

    pub summary: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StreamInfo {
    pub bitrate: u32,
    pub encoding: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct TrackBoundary {
    pub artist: String,
    pub title: String,
    pub artwork_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub is_streaming: bool,
    pub is_playing: bool,
    pub is_recording: bool,
    pub current_track: Option<TrackBoundary>,
    pub stream_status: String,
    pub current_channel: Option<ChannelInfo>,
}

pub async fn get_channel(
    channel_name: &str,
) -> Result<ChannelInfo, Box<dyn std::error::Error + Send + Sync>> {
    println!("🔍 Looking up channel ID for: {}", channel_name);

    // Fetch the channels list
    let response = reqwest::get("https://fluxmusic.api.radiosphere.io/channels").await?;

    if !response.status().is_success() {
        return Err(format!("Channels API error: {}", response.status()).into());
    }

    let res: ChannelResponse = response.json().await?;

    for channel in res.items.iter() {
        if channel.name == channel_name {
            return Ok(channel.clone());
        }
    }

    return Err(format!("Channel '{}' not found in API response", channel_name).into());
}

pub async fn fetch_current_track(
    api_url: &str,
) -> Result<TrackBoundary, Box<dyn std::error::Error + Send + Sync>> {
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
    let artwork_url = track_info["artwork"].as_str().map(|url| {
        if url.contains("fluxmusic.cdn.radiosphere.io") {
            format!("{}?type=large", url) // Get high quality artwork
        } else {
            url.to_string()
        }
    });

    Ok(TrackBoundary {
        artist,
        title,
        artwork_url,
    })
}

pub async fn track_info_task(
    track_tx: mpsc::Sender<TrackBoundary>,
    ui_tx: watch::Sender<AppState>,
) {
    println!("🎵 Track info task started");

    let channel_id: String;

    loop {
        let state = ui_tx.borrow().clone();

        match state.current_channel {
            Some(channel) => {
                channel_id = channel.channel_id;
                break;
            }
            None => {
                eprintln!("❌ Failed to get channel info, retrying in 2 seconds...");
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    }

    let api_url = format!(
        "https://fluxmusic.api.radiosphere.io/channels/{}/current-track",
        channel_id
    );
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
                    println!(
                        "🎵 Track changed: {} - {} (artwork: {})",
                        track_info.artist,
                        track_info.title,
                        track_info
                            .artwork_url
                            .as_ref()
                            .map(|_| "yes")
                            .unwrap_or("no")
                    );

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
