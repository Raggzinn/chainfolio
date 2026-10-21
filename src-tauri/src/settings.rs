//! Settings the backend needs even while the database is locked (tray
//! behaviour, sync schedule, notification language). Stored as JSON next to
//! the database; UI-only preferences stay in the frontend.

use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::error::Result;

/// Allowed background refresh intervals, in minutes; 0 turns it off.
pub const SYNC_INTERVALS: [u32; 6] = [0, 5, 15, 30, 60, 240];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// Closing the window hides it to the tray instead of quitting.
    pub close_to_tray: bool,
    /// Minutes between background refreshes; 0 turns them off.
    pub sync_interval_minutes: u32,
    /// Native notifications when an alert fires.
    pub notifications: bool,
    /// Language of tray labels and notifications ("en" or "pt-BR").
    pub locale: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            sync_interval_minutes: 15,
            notifications: true,
            locale: "en".into(),
        }
    }
}

impl AppSettings {
    /// Snaps out-of-range values (e.g. from an edited file or an old backup)
    /// back to something the app supports.
    pub fn sanitized(mut self) -> Self {
        if !SYNC_INTERVALS.contains(&self.sync_interval_minutes) {
            self.sync_interval_minutes = AppSettings::default().sync_interval_minutes;
        }
        if self.locale != "pt-BR" {
            self.locale = "en".into();
        }
        self
    }
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<AppSettings>,
}

impl SettingsStore {
    /// Reads `path`, falling back to defaults when it is missing or invalid.
    pub fn load(path: PathBuf) -> Self {
        let current = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<AppSettings>(&bytes).ok())
            .unwrap_or_default()
            .sanitized();
        Self {
            path,
            current: Mutex::new(current),
        }
    }

    pub fn get(&self) -> AppSettings {
        self.lock().clone()
    }

    pub fn set(&self, settings: AppSettings) -> Result<AppSettings> {
        let settings = settings.sanitized();
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(&settings).map_err(std::io::Error::other)?,
        )?;
        std::fs::rename(&tmp, &self.path)?;
        *self.lock() = settings.clone();
        Ok(settings)
    }

    fn lock(&self) -> MutexGuard<'_, AppSettings> {
        self.current
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Settings plus the OS-level launch-at-login flag.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    #[serde(flatten)]
    pub settings: AppSettings,
    pub launch_at_login: bool,
}

#[tauri::command]
pub fn get_app_settings(app: AppHandle, store: State<'_, SettingsStore>) -> SettingsView {
    SettingsView {
        settings: store.get(),
        launch_at_login: launch_at_login(&app),
    }
}

#[tauri::command]
pub fn update_app_settings(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    update: SettingsView,
) -> Result<SettingsView> {
    let previous = store.get();
    let settings = store.set(update.settings)?;
    if settings.locale != previous.locale {
        #[cfg(desktop)]
        crate::tray::relabel(&app, &settings.locale);
    }
    if update.launch_at_login != launch_at_login(&app) {
        set_launch_at_login(&app, update.launch_at_login)?;
    }
    Ok(SettingsView {
        settings,
        launch_at_login: launch_at_login(&app),
    })
}

#[cfg(desktop)]
fn launch_at_login(app: &AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[cfg(desktop)]
fn set_launch_at_login(app: &AppHandle, enabled: bool) -> Result<()> {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    result.map_err(|e| crate::error::Error::Io(std::io::Error::other(e.to_string())))
}

#[cfg(not(desktop))]
fn launch_at_login(_app: &AppHandle) -> bool {
    false
}

#[cfg(not(desktop))]
fn set_launch_at_login(_app: &AppHandle, _enabled: bool) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_or_broken_file_falls_back_to_defaults() {
        let dir = std::env::temp_dir().join(format!("chainfolio-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        assert_eq!(
            SettingsStore::load(path.clone()).get(),
            AppSettings::default()
        );

        std::fs::write(&path, b"{ not json").unwrap();
        assert_eq!(
            SettingsStore::load(path.clone()).get(),
            AppSettings::default()
        );

        let store = SettingsStore::load(path.clone());
        store
            .set(AppSettings {
                close_to_tray: false,
                sync_interval_minutes: 7,
                notifications: false,
                locale: "pt-BR".into(),
            })
            .unwrap();
        let reloaded = SettingsStore::load(path).get();
        std::fs::remove_dir_all(&dir).ok();
        assert!(!reloaded.close_to_tray);
        assert_eq!(reloaded.locale, "pt-BR");
        // 7 isn't an offered interval, so it snaps back to the default.
        assert_eq!(reloaded.sync_interval_minutes, 15);
    }

    #[test]
    fn unknown_fields_and_partial_files_are_accepted() {
        let parsed: AppSettings =
            serde_json::from_str(r#"{"syncIntervalMinutes": 60, "future": true}"#).unwrap();
        assert_eq!(parsed.sync_interval_minutes, 60);
        assert!(parsed.close_to_tray);
    }
}
