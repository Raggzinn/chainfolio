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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::Family,
        db::repo::{self, NewWallet, WalletKind},
    };

    #[test]
    fn wallets_persist_across_reopen() {
        let dir = std::env::temp_dir().join(format!("chainfolio-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("chainfolio.db");
        {
            let mut conn = open(&path).unwrap();
            repo::insert_wallet(
                &mut conn,
                &NewWallet {
                    label: "Cold",
                    family: Family::Bitcoin,
                    kind: WalletKind::Address,
                    address: "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
                    input: "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
                },
            )
            .unwrap();
        }
        // Reopening runs the migration runner again, which must be a no-op.
        let conn = open(&path).unwrap();
        let wallets = repo::list_wallets(&conn).unwrap();
        drop(conn);
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].label, "Cold");
    }
}
