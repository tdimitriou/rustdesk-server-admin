use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Error as SqlxError, SqlitePool};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PeerRow {
    pub guid: Vec<u8>,
    pub id: String,
    pub uuid: Vec<u8>,
    pub pk: Vec<u8>,
    pub user: Option<Vec<u8>>,
    pub created_at: String,
    pub status: Option<i64>,
    pub note: Option<String>,
    pub info: String,
}

pub async fn open_pool(db_path: &str) -> Result<SqlitePool, SqlxError> {
    let opts = SqliteConnectOptions::new()
        .filename(Path::new(db_path))
        .create_if_missing(false)
        .busy_timeout(Duration::from_secs(30));
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
}

pub async fn peer_count(pool: &SqlitePool) -> Result<i64, SqlxError> {
    let n: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM peer")
        .fetch_one(pool)
        .await?;
    Ok(n.unwrap_or(0))
}

/// Escape `%`, `_`, and `\` for use in SQLite LIKE with ESCAPE '\\'.
fn like_pattern(term: &str) -> String {
    let mut p = String::from("%");
    for c in term.chars() {
        if matches!(c, '%' | '_' | '\\') {
            p.push('\\');
        }
        p.push(c);
    }
    p.push('%');
    p
}

pub async fn list_peers(pool: &SqlitePool, search: &str) -> Result<Vec<PeerRow>, SqlxError> {
    let search = search.trim();
    if search.is_empty() {
        sqlx::query_as::<_, PeerRow>(
            "SELECT guid, id, uuid, pk, user, created_at, status, note, info \
             FROM peer ORDER BY id COLLATE NOCASE",
        )
        .fetch_all(pool)
        .await
    } else {
        let pat = like_pattern(search);
        sqlx::query_as::<_, PeerRow>(
            "SELECT guid, id, uuid, pk, user, created_at, status, note, info FROM peer \
             WHERE id LIKE ?1 ESCAPE '\\' \
                OR lower(hex(uuid)) LIKE lower(?1) ESCAPE '\\' \
                OR IFNULL(note, '') LIKE ?1 ESCAPE '\\' \
                OR info LIKE ?1 ESCAPE '\\' \
             ORDER BY id COLLATE NOCASE",
        )
        .bind(&pat)
        .fetch_all(pool)
        .await
    }
}

pub async fn delete_peer_by_guid(pool: &SqlitePool, guid: &[u8]) -> Result<u64, SqlxError> {
    let r = sqlx::query("DELETE FROM peer WHERE guid = ?")
        .bind(guid)
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

#[derive(Debug)]
pub enum UpdateIdError {
    NotFound,
    DuplicateId,
    Sql(SqlxError),
}

pub async fn update_peer_id(
    pool: &SqlitePool,
    guid: &[u8],
    new_id: &str,
) -> Result<(), UpdateIdError> {
    let res = sqlx::query("UPDATE peer SET id = ? WHERE guid = ?")
        .bind(new_id)
        .bind(guid)
        .execute(pool)
        .await;
    match res {
        Ok(r) if r.rows_affected() == 0 => Err(UpdateIdError::NotFound),
        Ok(_) => Ok(()),
        Err(e) if is_unique_violation(&e) => Err(UpdateIdError::DuplicateId),
        Err(e) => Err(UpdateIdError::Sql(e)),
    }
}

fn is_unique_violation(e: &SqlxError) -> bool {
    if let SqlxError::Database(d) = e {
        if d.code().as_deref() == Some("2067") {
            return true;
        }
        let m = d.message();
        return m.contains("UNIQUE") || m.contains("unique");
    }
    false
}
