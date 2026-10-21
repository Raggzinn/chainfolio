use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use rusqlite::Connection;
use serde::Serialize;

use crate::{
    db,
    error::{Error, Result},
};

pub struct AppState {
    /// `None` while an encrypted database is locked.
    db: Mutex<Option<Connection>>,
    db_path: PathBuf,
    encrypted: AtomicBool,
    pub http: reqwest::Client,
    /// Serializes refreshes so two syncs never race on the same rows.
    pub sync: tokio::sync::Mutex<()>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityStatus {
    pub encrypted: bool,
    pub locked: bool,
}

/// The open database connection; derefs to [`Connection`].
pub struct DbGuard<'a>(MutexGuard<'a, Option<Connection>>);

impl Deref for DbGuard<'_> {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        // `AppState::db` only hands out guards over an open connection.
        self.0.as_ref().expect("database is open")
    }
}

impl DerefMut for DbGuard<'_> {
    fn deref_mut(&mut self) -> &mut Connection {
        self.0.as_mut().expect("database is open")
    }
}

impl AppState {
    /// `conn` is `None` when the database at `db_path` is encrypted and
    /// waits for [`AppState::unlock`].
    pub fn new(conn: Option<Connection>, db_path: PathBuf) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "Chainfolio/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/Raggzinn/chainfolio)"
            ))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()?;
        let encrypted = conn.is_none() || db::is_encrypted(&db_path)?;
        Ok(Self {
            db: Mutex::new(conn),
            db_path,
            encrypted: AtomicBool::new(encrypted),
            http,
            sync: tokio::sync::Mutex::new(()),
        })
    }

    /// The database connection. Never hold the guard across an `.await`.
    pub fn db(&self) -> Result<DbGuard<'_>> {
        let guard = self.slot();
        if guard.is_none() {
            return Err(Error::Locked);
        }
        Ok(DbGuard(guard))
    }

    fn slot(&self) -> MutexGuard<'_, Option<Connection>> {
        // A panic while holding the lock can't leave SQLite half-written
        // (writes are transactional), so a poisoned lock is safe to reuse.
        self.db
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn security_status(&self) -> SecurityStatus {
        SecurityStatus {
            encrypted: self.encrypted.load(Ordering::Relaxed),
            locked: self.slot().is_none(),
        }
    }

    pub fn unlock(&self, password: &str) -> Result<()> {
        let mut slot = self.slot();
        if slot.is_none() {
            *slot = Some(db::open(&self.db_path, Some(password))?);
        }
        Ok(())
    }

    /// Closes an encrypted database so its key leaves memory. A plaintext
    /// database has nothing to protect and stays open.
    pub fn lock(&self) {
        if self.encrypted.load(Ordering::Relaxed) {
            self.slot().take();
        }
    }

    /// Sets, changes (`new` = `Some`) or removes (`new` = `None`) the master
    /// password. `current` must match when the database is encrypted.
    pub fn change_key(&self, current: Option<&str>, new: Option<&str>) -> Result<()> {
        if new.is_some_and(|p| p.chars().count() < 8) {
            return Err(Error::WeakPassword);
        }
        let encrypted = self.encrypted.load(Ordering::Relaxed);
        let current = if encrypted { current } else { None };
        if encrypted && current.is_none() {
            return Err(Error::WrongPassword);
        }

        let mut slot = self.slot();
        let conn = slot.as_ref().ok_or(Error::Locked)?;
        db::verify_key(&self.db_path, current)?;
        let copy = db::rekeyed_copy(conn, &self.db_path, new)?;
        slot.take();
        let swapped = db::replace(&copy, &self.db_path);
        let key = if swapped.is_ok() { new } else { current };
        *slot = Some(db::open(&self.db_path, key)?);
        swapped?;
        self.encrypted.store(new.is_some(), Ordering::Relaxed);
        Ok(())
    }
}
