use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct ShareRow {
    pub share_key: String,
    pub kind: String,
    pub raw_file_name: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
}

pub fn open(db_path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS shares (
            share_key TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            raw_file_name TEXT,
            is_public INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL
        );",
    )?;
    Ok(conn)
}

pub fn active_count(conn: &Connection, now: i64) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM shares WHERE expires_at >= ?1",
        params![now],
        |r| r.get(0),
    )
}

pub fn key_exists(conn: &Connection, key: &str) -> rusqlite::Result<bool> {
    let exists: Option<i64> = conn
        .query_row("SELECT 1 FROM shares WHERE share_key = ?1", params![key], |r| r.get(0))
        .optional()?;
    Ok(exists.is_some())
}

pub fn upsert_share(
    conn: &Connection,
    share_key: &str,
    kind: &str,
    raw_file_name: Option<&str>,
    is_public: bool,
    created_at: i64,
    expires_at: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO shares (share_key, kind, raw_file_name, is_public, created_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(share_key) DO UPDATE SET
            kind = excluded.kind,
            raw_file_name = excluded.raw_file_name,
            is_public = excluded.is_public,
            created_at = excluded.created_at,
            expires_at = excluded.expires_at",
        params![share_key, kind, raw_file_name, is_public as i64, created_at, expires_at],
    )?;
    Ok(())
}

pub fn get_share(conn: &Connection, key: &str, now: i64) -> rusqlite::Result<Option<ShareRow>> {
    conn.query_row(
        "SELECT share_key, kind, raw_file_name, created_at, expires_at
         FROM shares WHERE share_key = ?1 AND expires_at >= ?2",
        params![key, now],
        |r| {
            Ok(ShareRow {
                share_key: r.get(0)?,
                kind: r.get(1)?,
                raw_file_name: r.get(2)?,
                created_at: r.get(3)?,
                expires_at: r.get(4)?,
            })
        },
    )
    .optional()
}

pub fn list_public(conn: &Connection, now: i64) -> rusqlite::Result<Vec<ShareRow>> {
    let mut stmt = conn.prepare(
        "SELECT share_key, kind, raw_file_name, created_at, expires_at
         FROM shares WHERE is_public = 1 AND expires_at >= ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt
        .query_map(params![now], |r| {
            Ok(ShareRow {
                share_key: r.get(0)?,
                kind: r.get(1)?,
                raw_file_name: r.get(2)?,
                created_at: r.get(3)?,
                expires_at: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn delete_share(conn: &Connection, key: &str) -> rusqlite::Result<bool> {
    let affected = conn.execute("DELETE FROM shares WHERE share_key = ?1", params![key])?;
    Ok(affected > 0)
}

pub fn expired_keys(conn: &Connection, now: i64) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT share_key FROM shares WHERE expires_at < ?1")?;
    let rows = stmt
        .query_map(params![now], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn delete_expired(conn: &Connection, now: i64) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM shares WHERE expires_at < ?1", params![now])
}
