use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;

pub async fn peer_count(db_path: &str) -> Result<i64, sqlx::Error> {
    let opts = SqliteConnectOptions::new()
        .filename(Path::new(db_path))
        .read_only(true)
        .create_if_missing(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    let n: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM peer")
        .fetch_one(&pool)
        .await?;
    pool.close().await;
    Ok(n.unwrap_or(0))
}
