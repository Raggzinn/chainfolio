//! Keeps balances and prices fresh while the app runs, including when it is
//! hidden in the tray.

use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::Duration,
};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::{
    alerts::{self, Condition, Trigger},
    db,
    error::Result,
    settings::SettingsStore,
    state::AppState,
    sync::{self, RefreshReport},
    text,
};

/// Emitted with the [`RefreshReport`] after every sync, wherever it started.
pub const EVENT_SYNCED: &str = "sync://completed";
/// Emitted with the new [`Trigger`]s when alerts fire.
pub const EVENT_ALERTS: &str = "alerts://triggered";

/// More triggers than this in one sync collapse into a single notification.
const MAX_NOTIFICATIONS: usize = 3;

/// Serializes whole cycles (snapshot, refresh, alerts) so every balance
/// change is attributed to exactly one sync.
static CYCLE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// When the scheduler last tried to sync, so a failing network isn't retried
/// every minute.
static LAST_ATTEMPT: AtomicI64 = AtomicI64::new(0);

/// Refreshes one wallet (or all), evaluates alerts against the result and
/// tells the window about both.
pub async fn sync_now(app: &AppHandle, wallet_id: Option<i64>) -> Result<RefreshReport> {
    let _cycle = CYCLE.lock().await;
    let state = app.state::<AppState>();
    let before = alerts::store::snapshot(&*state.db()?)?;
    let report = sync::refresh(&state, wallet_id).await?;
    if let Err(err) = alerts::price_watched(&state).await {
        eprintln!("pricing watched assets failed: {err}");
    }
    let triggers = alerts::check(&mut *state.db()?, &before, &report.portfolio)?;
    let _ = app.emit(EVENT_SYNCED, &report);
    if !triggers.is_empty() {
        notify(app, &triggers);
        let _ = app.emit(EVENT_ALERTS, &triggers);
    }
    Ok(report)
}

fn notify(app: &AppHandle, triggers: &[Trigger]) {
    let settings = app.state::<SettingsStore>().get();
    if !settings.notifications {
        return;
    }
    #[cfg(desktop)]
    let hidden = crate::tray::privacy_on(app);
    #[cfg(not(desktop))]
    let hidden = false;
    let state = app.state::<AppState>();
    let label = |trigger: &Trigger| -> Option<String> {
        let Condition::BalanceChange { .. } = trigger.condition else {
            return None;
        };
        let id: i64 = trigger.subject.as_deref()?.parse().ok()?;
        let db = state.db().ok()?;
        db::repo::get_wallet(&db, id).ok().map(|w| w.label)
    };
    let messages: Vec<(String, String)> = triggers
        .iter()
        .map(|t| text::notification(&settings.locale, t, label(t).as_deref(), hidden))
        .collect();
    let show = |(title, body): (String, String)| {
        if let Err(err) = app.notification().builder().title(title).body(body).show() {
            eprintln!("notification failed: {err}");
        }
    };
    if messages.len() > MAX_NOTIFICATIONS {
        let titles: Vec<String> = messages.into_iter().map(|(title, _)| title).collect();
        show(text::notification_summary(&settings.locale, &titles));
    } else {
        messages.into_iter().for_each(show);
    }
}

/// Starts the scheduler. The first run waits a full interval because the
/// frontend already refreshes stale balances on launch.
pub fn spawn(app: AppHandle) {
    LAST_ATTEMPT.store(db::now_ms(), Ordering::Relaxed);
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if !should_run(&app) {
                continue;
            }
            LAST_ATTEMPT.store(db::now_ms(), Ordering::Relaxed);
            if let Err(err) = sync_now(&app, None).await {
                eprintln!("background sync failed: {err}");
            }
        }
    });
}

fn should_run(app: &AppHandle) -> bool {
    let interval = app.state::<SettingsStore>().get().sync_interval_minutes;
    let state = app.state::<AppState>();
    // Locked databases can't be read; a running refresh will be fresh enough.
    if interval == 0 || state.sync.try_lock().is_err() {
        return false;
    }
    let Ok(db) = state.db() else {
        return false;
    };
    let Ok((wallets, synced_at)) =
        db.query_row("SELECT COUNT(*), MAX(synced_at) FROM wallets", [], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?))
        })
    else {
        return false;
    };
    wallets > 0
        && is_due(
            db::now_ms(),
            synced_at,
            LAST_ATTEMPT.load(Ordering::Relaxed),
            interval,
        )
}

/// True once `interval_minutes` have passed since both the last successful
/// sync and the last attempt.
fn is_due(now: i64, synced_at: Option<i64>, last_attempt: i64, interval_minutes: u32) -> bool {
    let last = synced_at.unwrap_or(0).max(last_attempt);
    now - last >= i64::from(interval_minutes) * 60_000
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;

    #[test]
    fn due_after_the_interval() {
        let now = 1_000 * MIN;
        assert!(!is_due(now, Some(now - 14 * MIN), 0, 15));
        assert!(is_due(now, Some(now - 15 * MIN), 0, 15));
        // Never synced: run as soon as the last attempt is old enough.
        assert!(is_due(now, None, now - 20 * MIN, 15));
    }

    #[test]
    fn failed_attempts_back_off_for_a_full_interval() {
        let now = 1_000 * MIN;
        assert!(!is_due(now, Some(now - 60 * MIN), now - 5 * MIN, 15));
        assert!(is_due(now, Some(now - 60 * MIN), now - 15 * MIN, 15));
    }
}
