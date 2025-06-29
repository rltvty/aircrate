use sqlx::{Pool, Sqlite};
use super::schema::*;

pub async fn run_migrations(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    println!("Running database migrations...");

    // Create all tables
    sqlx::query(CREATE_TRACKS_TABLE).execute(pool).await?;
    sqlx::query(CREATE_RELEASES_TABLE).execute(pool).await?;
    sqlx::query(CREATE_ARTISTS_TABLE).execute(pool).await?;
    sqlx::query(CREATE_TRACK_ARTISTS_TABLE).execute(pool).await?;
    sqlx::query(CREATE_EXTERNAL_LINKS_TABLE).execute(pool).await?;
    sqlx::query(CREATE_ARTIST_IMAGES_TABLE).execute(pool).await?;
    sqlx::query(CREATE_RELEASE_IMAGES_TABLE).execute(pool).await?;

    // Create indexes
    for index_sql in CREATE_INDEXES {
        sqlx::query(index_sql).execute(pool).await?;
    }

    println!("Database migrations completed successfully!");
    Ok(())
}