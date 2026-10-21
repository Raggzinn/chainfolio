//! SQLite storage. Everything lives in one file in the app data directory.

pub mod history;
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
    conn.pragma_update(None, "journal_mode", "WAL")?;
    // Migrations that rebuild a parent table must not cascade deletes into
    // its children, so foreign keys are only enforced once they have run.
    conn.pragma_update(None, "foreign_keys", false)?;
    embedded::migrations::runner().run(&mut conn)?;
    conn.pragma_update(None, "foreign_keys", true)?;
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

    #[test]
    fn v2_keeps_wallets_and_their_balances() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        embedded::migrations::runner()
            .set_target(refinery::Target::Version(1))
            .run(&mut conn)
            .unwrap();
        conn.execute_batch(
            "INSERT INTO wallets (label, family, kind, address, input, created_at)
               VALUES ('Main', 'evm', 'address', '0xa', '0xa', 1);
             INSERT INTO wallet_addresses (wallet_id, address) VALUES (1, '0xa');
             INSERT INTO assets (id, chain, symbol, name, decimals, price_key)
               VALUES ('ethereum:native', 'ethereum', 'ETH', 'Ether', 18, 'coingecko:ethereum');
             INSERT INTO balances (wallet_id, asset_id, amount, updated_at)
               VALUES (1, 'ethereum:native', '1.5', 1);",
        )
        .unwrap();

        let conn = prepare(conn).unwrap();
        let count = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(count("wallets"), 1);
        assert_eq!(count("wallet_addresses"), 1);
        assert_eq!(count("balances"), 1);
        // New families are accepted and cascades still work after the rebuild.
        conn.execute(
            "INSERT INTO wallets (label, family, kind, address, input, created_at)
             VALUES ('Tron', 'tron', 'address', 'T1', 'T1', 2)",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM wallets WHERE id = 1", [])
            .unwrap();
        assert_eq!(count("balances"), 0);
        assert_eq!(count("wallet_addresses"), 0);
    }
}
