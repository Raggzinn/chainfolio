//! System tray icon: reopen the window, sync, toggle privacy mode, quit.

use tauri::{
    AppHandle, Emitter, Manager, State, Wry,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::{background, settings::SettingsStore, text};

/// Emitted with the new privacy flag when "Hide balances" is clicked.
pub const EVENT_PRIVACY: &str = "tray://privacy";

/// Menu items whose label or check mark changes at runtime.
pub struct TrayMenu {
    open: MenuItem<Wry>,
    sync: MenuItem<Wry>,
    privacy: CheckMenuItem<Wry>,
    quit: MenuItem<Wry>,
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let s = text::strings(&app.state::<SettingsStore>().get().locale);
    let open = MenuItem::with_id(app, "open", s.open, true, None::<&str>)?;
    let sync = MenuItem::with_id(app, "sync", s.sync_now, true, None::<&str>)?;
    let privacy =
        CheckMenuItem::with_id(app, "privacy", s.hide_balances, true, false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", s.quit, true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &sync,
            &PredefinedMenuItem::separator(app)?,
            &privacy,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Chainfolio")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "sync" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = background::sync_now(&app, None).await {
                        eprintln!("tray sync failed: {err}");
                    }
                });
            }
            "privacy" => {
                let menu = app.state::<TrayMenu>();
                if let Ok(hidden) = menu.privacy.is_checked() {
                    let _ = app.emit(EVENT_PRIVACY, hidden);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    app.manage(TrayMenu {
        open,
        sync,
        privacy,
        quit,
    });
    Ok(())
}

/// Brings the main window back from the tray (or from behind other windows).
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Re-translates the menu after the UI language changes.
pub fn relabel(app: &AppHandle, locale: &str) {
    let Some(menu) = app.try_state::<TrayMenu>() else {
        return;
    };
    let s = text::strings(locale);
    let _ = menu.open.set_text(s.open);
    let _ = menu.sync.set_text(s.sync_now);
    let _ = menu.privacy.set_text(s.hide_balances);
    let _ = menu.quit.set_text(s.quit);
}

/// Whether "Hide balances" is on, so notifications leave amounts out.
pub fn privacy_on(app: &AppHandle) -> bool {
    app.try_state::<TrayMenu>()
        .and_then(|menu| menu.privacy.is_checked().ok())
        .unwrap_or(false)
}

/// Mirrors privacy mode toggled from inside the app.
#[tauri::command]
pub fn set_tray_privacy(menu: State<'_, TrayMenu>, hidden: bool) {
    let _ = menu.privacy.set_checked(hidden);
}
