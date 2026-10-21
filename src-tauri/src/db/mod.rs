//! SQLite storage. Everything lives in one file in the app data directory,
//! optionally encrypted with SQLCipher under the user's master password.

pub mod repo;

use std::path::{Path, PathBuf};

use rusqlite::{Connection, ErrorCode, OpenFlags};

use crate::error::{Error, Result};

mod embedded {
    refinery::embed_migrations!("migrations");
}

/// Opens (or creates) the database at `path` and applies pending migrations.
/// `key` is the master password of an encrypted database.
pub fn open(path: &Path, key: Option<&str>) -> Result<Connection> {
    let conn = Connection::open(path)?;
    if let Some(key) = key {
        conn.pragma_update(None, "key", key)?;
    }
    // Reading the schema is the first access that needs the key.
    if not_a_database(schema_check(&conn))? {
        return Err(if key.is_some() {
            Error::WrongPassword
        } else {
            Error::Locked
        });
    }
    prepare(conn)
}

/// True when the file exists and can't be read without a key.
pub fn is_encrypted(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    not_a_database(schema_check(&conn))
}

/// Checks `key` against the database at `path` without touching the open connection.
pub fn verify_key(path: &Path, key: Option<&str>) -> Result<()> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    if let Some(key) = key {
        conn.pragma_update(None, "key", key)?;
    }
    if not_a_database(schema_check(&conn))? {
        return Err(Error::WrongPassword);
    }
    Ok(())
}

/// Writes a copy of the database at `path` re-encrypted under `new_key`
/// (`None` = plaintext) and returns where it went. The original stays
/// untouched until [`replace`] swaps the copy in.
pub fn rekeyed_copy(conn: &Connection, path: &Path, new_key: Option<&str>) -> Result<PathBuf> {
    let copy = sibling(path, "rekey");
    remove_with_sidecars(&copy)?;
    conn.execute(
        "ATTACH DATABASE ?1 AS rekeyed KEY ?2",
        (copy.to_string_lossy(), new_key.unwrap_or("")),
    )?;
    let exported = conn.query_row("SELECT sqlcipher_export('rekeyed')", [], |_| Ok(()));
    conn.execute("DETACH DATABASE rekeyed", [])?;
    exported?;
    Ok(copy)
}

/// Moves a copy made by [`rekeyed_copy`] over `path`. Every connection to
/// `path` must be closed first.
pub fn replace(copy: &Path, path: &Path) -> Result<()> {
    // Closing the last connection checkpoints the WAL; leftovers would belong
    // to the old file and corrupt the new one.
    for sidecar in [sibling(path, "db-wal"), sibling(path, "db-shm")] {
        if sidecar.exists() {
            std::fs::remove_file(sidecar)?;
        }
    }
    std::fs::rename(copy, path)?;
    Ok(())
}

fn schema_check(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0))
}

fn not_a_database(result: rusqlite::Result<i64>) -> Result<bool> {
    match result {
        Ok(_) => Ok(false),
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == ErrorCode::NotADatabase => Ok(true),
        Err(e) => Err(e.into()),
    }
}

/// `chainfolio.db` -> `chainfolio.<ext>` in the same directory.
fn sibling(path: &Path, ext: &str) -> PathBuf {
    path.with_extension(ext)
}

fn remove_with_sidecars(path: &Path) -> Result<()> {
    let name = path.as_os_str().to_owned();
    for suffix in ["", "-journal", "-wal", "-shm"] {
        let mut file = name.clone();
        file.push(suffix);
        let file = PathBuf::from(file);
        if file.exists() {
            std::fs::remove_file(file)?;
        }
    }
    Ok(())
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

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chainfolio-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn add_cold_wallet(conn: &mut Connection) {
        repo::insert_wallet(
            conn,
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

    /// Re-encrypts like `AppState::change_key`: copy, close, swap, reopen.
    fn rekey(conn: Connection, path: &Path, new_key: Option<&str>) -> Connection {
        let copy = rekeyed_copy(&conn, path, new_key).unwrap();
        drop(conn);
        replace(&copy, path).unwrap();
        open(path, new_key).unwrap()
    }

    #[test]
    fn master_password_encrypts_rekeys_and_decrypts() {
        let dir = temp_dir("cipher");
        let path = dir.join("chainfolio.db");
        let mut conn = open(&path, None).unwrap();
        add_cold_wallet(&mut conn);
        assert!(!is_encrypted(&path).unwrap());

        let conn = rekey(conn, &path, Some("correct horse"));
        assert!(is_encrypted(&path).unwrap());
        assert!(matches!(open(&path, None), Err(Error::Locked)));
        assert!(matches!(
            open(&path, Some("wrong")),
            Err(Error::WrongPassword)
        ));
        assert!(matches!(
            verify_key(&path, Some("wrong")),
            Err(Error::WrongPassword)
        ));
        verify_key(&path, Some("correct horse")).unwrap();
        assert_eq!(repo::list_wallets(&conn).unwrap().len(), 1);

        // Quotes in the password must not break the PRAGMA or the ATTACH.
        let conn = rekey(conn, &path, Some("it's a \"new\" one"));
        assert!(matches!(
            open(&path, Some("correct horse")),
            Err(Error::WrongPassword)
        ));
        assert_eq!(repo::list_wallets(&conn).unwrap().len(), 1);

        let conn = rekey(conn, &path, None);
        assert!(!is_encrypted(&path).unwrap());
        assert_eq!(repo::list_wallets(&conn).unwrap()[0].label, "Cold");
        drop(conn);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn wallets_persist_across_reopen() {
        let dir = temp_dir("reopen");
        let path = dir.join("chainfolio.db");
        add_cold_wallet(&mut open(&path, None).unwrap());
        // Reopening runs the migration runner again, which must be a no-op.
        let conn = open(&path, None).unwrap();
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
