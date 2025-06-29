// Database schema definitions for rich track metadata

// Main tracks table - stores basic track info
pub const CREATE_TRACKS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS tracks (
    track_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    artist_credits TEXT NOT NULL,
    artwork_url TEXT,
    raw_metadata TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;

// Releases table - stores album/release information
pub const CREATE_RELEASES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS releases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    title TEXT NOT NULL,
    year INTEGER,
    label TEXT,
    artwork_url TEXT,
    raw_metadata TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (track_id) REFERENCES tracks (track_id)
);
"#;

// Artists table - stores detailed artist information
pub const CREATE_ARTISTS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS artists (
    artist_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    about_content TEXT,
    about_source TEXT,
    country TEXT,
    raw_metadata TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;

// Junction table linking tracks to artists (many-to-many)
pub const CREATE_TRACK_ARTISTS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS track_artists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    artist_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (track_id) REFERENCES tracks (track_id),
    FOREIGN KEY (artist_id) REFERENCES artists (artist_id),
    UNIQUE(track_id, artist_id)
);
"#;

// External links for tracks, releases, and artists
pub const CREATE_EXTERNAL_LINKS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS external_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL, -- 'track', 'release', 'artist'
    entity_id TEXT NOT NULL,
    link_type TEXT NOT NULL, -- 'Discogs', 'Facebook', 'Instagram', etc.
    url TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(entity_type, entity_id, link_type, url)
);
"#;

// Artist images - stores both URLs and local cached paths
pub const CREATE_ARTIST_IMAGES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS artist_images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artist_id TEXT NOT NULL,
    original_url TEXT NOT NULL,
    cached_path TEXT, -- Local filesystem path to cached image
    size_type TEXT, -- 'thumbnail', 'small', 'medium', 'large', 'original'
    license TEXT,
    source TEXT,
    thumbnail_data TEXT, -- Base64 encoded thumbnail from API
    created_at TEXT NOT NULL,
    FOREIGN KEY (artist_id) REFERENCES artists (artist_id)
);
"#;

// Release images - similar structure for release artwork
pub const CREATE_RELEASE_IMAGES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS release_images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    original_url TEXT NOT NULL,
    cached_path TEXT, -- Local filesystem path to cached image
    size_type TEXT, -- 'thumbnail', 'small', 'medium', 'large', 'original'
    license TEXT,
    source TEXT,
    thumbnail_data TEXT, -- Base64 encoded thumbnail from API
    created_at TEXT NOT NULL,
    FOREIGN KEY (track_id) REFERENCES tracks (track_id)
);
"#;

// Index for better query performance
pub const CREATE_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks (title);",
    "CREATE INDEX IF NOT EXISTS idx_tracks_artist_credits ON tracks (artist_credits);",
    "CREATE INDEX IF NOT EXISTS idx_releases_track_id ON releases (track_id);",
    "CREATE INDEX IF NOT EXISTS idx_artists_name ON artists (name);",
    "CREATE INDEX IF NOT EXISTS idx_track_artists_track_id ON track_artists (track_id);",
    "CREATE INDEX IF NOT EXISTS idx_track_artists_artist_id ON track_artists (artist_id);",
    "CREATE INDEX IF NOT EXISTS idx_external_links_entity ON external_links (entity_type, entity_id);",
    "CREATE INDEX IF NOT EXISTS idx_artist_images_artist_id ON artist_images (artist_id);",
];