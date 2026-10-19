//! SQLite storage. Everything lives in one file in the app data directory.

pub mod repo;

use std::path::Path;

use rusqlite::Connection;

use crate::error::Result;

mod embedded {
    refinery::embed_migrations!("migrations");
}

/// Opens (or creates) the database at `path` and applies pending migrations.
pub fn open(path: &Path) -> Result<Connection> {
    prepare(Connection::open(path)?)
}

/// Fresh in-memory database with the full schema, for tests.
#[cfg(test)]
pub fn open_in_memory() -> Result<Connection> {
    prepare(Connection::open_in_memory()?)
}

fn prepare(mut conn: Connection) -> Result<Connection> {
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    embedded::migrations::runner().run(&mut conn)?;
    Ok(conn)
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}
