use std::{
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use rusqlite::Connection;

use crate::error::Result;

pub struct AppState {
    db: Mutex<Connection>,
    pub http: reqwest::Client,
    /// Serializes refreshes so two syncs never race on the same rows.
    pub sync: tokio::sync::Mutex<()>,
}

impl AppState {
    pub fn new(conn: Connection) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "Chainfolio/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/Raggzinn/chainfolio)"
            ))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self {
            db: Mutex::new(conn),
            http,
            sync: tokio::sync::Mutex::new(()),
        })
    }

    /// The database connection. Never hold the guard across an `.await`.
    pub fn db(&self) -> MutexGuard<'_, Connection> {
        // A panic while holding the lock can't leave SQLite half-written
        // (writes are transactional), so a poisoned lock is safe to reuse.
        self.db
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
