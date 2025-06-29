use sqlx::{Pool, Sqlite, SqlitePool};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod schema;
pub mod migrations;

pub use schema::*;

#[derive(Debug, Clone, Copy)]
pub enum ImageSize {
    Thumbnail,
    Small,
    Medium,
    Large,
    Original,
}

#[derive(Clone)]
pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(database_url).await?;
        
        // Run migrations
        migrations::run_migrations(&pool).await?;
        
        Ok(Database { pool })
    }

    pub fn enhance_image_url(url: &str, size: ImageSize) -> String {
        let size_param = match size {
            ImageSize::Thumbnail => "thumbnail",
            ImageSize::Small => "small", 
            ImageSize::Medium => "medium",
            ImageSize::Large => "large",
            ImageSize::Original => return url.to_string(),
        };
        
        if url.contains("fluxmusic.cdn.radiosphere.io") {
            format!("{}?type={}", url, size_param)
        } else {
            url.to_string()
        }
    }

    pub async fn download_and_cache_image(&self, url: &str, cache_dir: &str) -> Result<String, Box<dyn std::error::Error>> {
        use std::path::Path;
        
        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(cache_dir)?;
        
        // Generate a filename from the URL
        let url_hash = format!("{:x}", md5::compute(url.as_bytes()));
        let extension = url.rsplit('.').next().unwrap_or("jpg");
        let filename = format!("{}.{}", url_hash, extension);
        let file_path = Path::new(cache_dir).join(&filename);
        
        // Check if file already exists
        if file_path.exists() {
            return Ok(file_path.to_string_lossy().to_string());
        }
        
        // Download the image
        println!("Downloading image: {}", url);
        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        
        // Save to filesystem
        std::fs::write(&file_path, &bytes)?;
        println!("Cached image: {}", file_path.display());
        
        Ok(file_path.to_string_lossy().to_string())
    }

    pub async fn store_track_metadata(&self, track_data: &serde_json::Value) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Parse the track info
        let track_info = &track_data["trackInfo"];
        let track_id = track_info["trackId"].as_str().unwrap_or_default();
        let title = track_info["title"].as_str().unwrap_or_default();
        let artist_credits = track_info["artistCredits"].as_str().unwrap_or_default();
        let artwork_url = track_info["artwork"].as_str();

        // Store or update track
        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO tracks (
                track_id, title, artist_credits, artwork_url, 
                raw_metadata, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))
            "#,
            track_id,
            title,
            artist_credits,
            artwork_url,
            track_data.to_string()
        )
        .execute(&mut *tx)
        .await?;

        // Store release info if available
        if let Some(release) = track_info["release"].as_object() {
            let release_title = release["title"].as_str().unwrap_or_default();
            let release_year = release["year"].as_i64();
            let release_label = release["label"].as_str();
            let release_artwork_url = release.get("artwork")
                .and_then(|a| a["url"].as_str());

            sqlx::query!(
                r#"
                INSERT OR REPLACE INTO releases (
                    track_id, title, year, label, artwork_url, 
                    raw_metadata, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
                "#,
                track_id,
                release_title,
                release_year,
                release_label,
                release_artwork_url,
                serde_json::to_string(release).unwrap_or_default()
            )
            .execute(&mut *tx)
            .await?;

            // Store external links for release
            if let Some(external_links) = release["externalLinks"].as_array() {
                for link in external_links {
                    if let (Some(link_type), Some(url)) = (link["type"].as_str(), link["url"].as_str()) {
                        sqlx::query!(
                            r#"
                            INSERT OR REPLACE INTO external_links (
                                entity_type, entity_id, link_type, url, created_at
                            ) VALUES ('release', ?, ?, ?, datetime('now'))
                            "#,
                            track_id,
                            link_type,
                            url
                        )
                        .execute(&mut *tx)
                        .await?;
                    }
                }
            }
        }

        // Store artist info
        if let Some(artists) = track_info["artists"].as_array() {
            for artist in artists {
                let artist_name = artist["name"].as_str().unwrap_or_default();
                let artist_id = Uuid::new_v4().to_string(); // Generate unique ID for artist
                let about_content = artist.get("about")
                    .and_then(|a| a["content"].as_str());
                let about_source = artist.get("about")
                    .and_then(|a| a["sourceUrl"].as_str());
                let country = artist["country"].as_str();

                sqlx::query!(
                    r#"
                    INSERT OR REPLACE INTO artists (
                        artist_id, name, about_content, about_source, country, 
                        raw_metadata, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
                    "#,
                    artist_id,
                    artist_name,
                    about_content,
                    about_source,
                    country,
                    serde_json::to_string(artist).unwrap_or_default()
                )
                .execute(&mut *tx)
                .await?;

                // Link artist to track
                sqlx::query!(
                    r#"
                    INSERT OR REPLACE INTO track_artists (track_id, artist_id, created_at)
                    VALUES (?, ?, datetime('now'))
                    "#,
                    track_id,
                    artist_id
                )
                .execute(&mut *tx)
                .await?;

                // Store artist external links
                if let Some(external_links) = artist["externalLinks"].as_array() {
                    for link in external_links {
                        if let (Some(link_type), Some(url)) = (link["type"].as_str(), link["url"].as_str()) {
                            sqlx::query!(
                                r#"
                                INSERT OR REPLACE INTO external_links (
                                    entity_type, entity_id, link_type, url, created_at
                                ) VALUES ('artist', ?, ?, ?, datetime('now'))
                                "#,
                                artist_id,
                                link_type,
                                url
                            )
                            .execute(&mut *tx)
                            .await?;
                        }
                    }
                }

                // Store artist images
                if let Some(images) = artist["images"].as_array() {
                    for image in images {
                        if let Some(image_url) = image["url"].as_str() {
                            let license = image["license"].as_str();
                            let source = image["source"].as_str();
                            let thumbnail = image["thumbnail"].as_str();

                            sqlx::query!(
                                r#"
                                INSERT OR REPLACE INTO artist_images (
                                    artist_id, url, license, source, thumbnail, created_at
                                ) VALUES (?, ?, ?, ?, ?, datetime('now'))
                                "#,
                                artist_id,
                                image_url,
                                license,
                                source,
                                thumbnail
                            )
                            .execute(&mut *tx)
                            .await?;
                        }
                    }
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_track_by_id(&self, track_id: &str) -> Result<Option<TrackRecord>, sqlx::Error> {
        let record = sqlx::query_as!(
            TrackRecord,
            "SELECT * FROM tracks WHERE track_id = ?",
            track_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    pub async fn get_artists_for_track(&self, track_id: &str) -> Result<Vec<ArtistRecord>, sqlx::Error> {
        let records = sqlx::query_as!(
            ArtistRecord,
            r#"
            SELECT a.* FROM artists a
            JOIN track_artists ta ON a.artist_id = ta.artist_id
            WHERE ta.track_id = ?
            "#,
            track_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackRecord {
    pub track_id: String,
    pub title: String,
    pub artist_credits: String,
    pub artwork_url: Option<String>,
    pub raw_metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistRecord {
    pub artist_id: String,
    pub name: String,
    pub about_content: Option<String>,
    pub about_source: Option<String>,
    pub country: Option<String>,
    pub raw_metadata: String,
    pub created_at: String,
    pub updated_at: String,
}