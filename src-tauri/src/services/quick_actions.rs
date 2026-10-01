use crate::services::cli_manager;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const OPENED_EVENT: &str = "quick-actions-opened";
pub const WINDOW_LABEL: &str = "quick-actions";
static OPEN_LOCK: Mutex<()> = Mutex::new(());

#[derive(Default)]
pub struct QuickActionsState(Mutex<Registration>);

#[derive(Default)]
struct Registration {
    shortcut: Option<Shortcut>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub enabled: bool,
    pub shortcut: String,
    pub error: Option<String>,
}

pub fn default_shortcut() -> &'static str {
    if cfg!(target_os = "macos") {
        "Command+Shift+K"
    } else {
        "Control+Shift+K"
    }
}

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _, event| {
            if event.state() == ShortcutState::Pressed {
                request_open(app);
            }
        })
        .build()
}

pub fn request_open(app: &tauri::AppHandle) {
    let app = app.clone();
    // WebView2 cannot create a window inside a synchronous event handler on Windows.
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = open(&app) {
            log::warn!("Quick Actions: {e}");
        }
    });
}

fn open(app: &tauri::AppHandle) -> Result<(), String> {
    let _opening = OPEN_LOCK.lock().map_err(|e| e.to_string())?;
    let window = match app.get_webview_window(WINDOW_LABEL) {
        Some(window) => window,
        None => tauri::WebviewWindowBuilder::new(
            app,
            WINDOW_LABEL,
            tauri::WebviewUrl::App("index.html?quick-actions=1".into()),
        )
        .title("Quick Actions")
        .inner_size(620.0, 520.0)
        .resizable(false)
        .always_on_top(true)
        .center()
        .build()
        .map_err(|e| e.to_string())?,
    };
    window.show().map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    window.emit(OPENED_EVENT, ()).map_err(|e| e.to_string())
}

pub fn settings(app: &tauri::AppHandle) -> Result<Settings, String> {
    let meta = cli_manager::load_meta();
    let state = app.state::<QuickActionsState>();
    let registration = state.0.lock().map_err(|e| e.to_string())?;
    Ok(Settings {
        enabled: meta.quick_actions_enabled.unwrap_or(true),
        shortcut: meta
            .quick_actions_shortcut
            .unwrap_or_else(|| default_shortcut().to_string()),
        error: registration.error.clone(),
    })
}

pub fn configure(app: &tauri::AppHandle, enabled: bool, shortcut: &str) -> Result<(), String> {
    let parsed = shortcut.parse::<Shortcut>().map_err(|e| e.to_string())?;
    let state = app.state::<QuickActionsState>();
    let mut registration = state.0.lock().map_err(|e| e.to_string())?;
    let next = enabled.then_some(parsed);
    let old = registration.shortcut;
    if next != old {
        if let Some(next) = next {
            app.global_shortcut()
                .register(next)
                .map_err(|e| format!("Could not register shortcut: {e}"))?;
        }
        if let Some(old) = old {
            if let Err(e) = app.global_shortcut().unregister(old) {
                if let Some(next) = next {
                    let _ = app.global_shortcut().unregister(next);
                }
                return Err(e.to_string());
            }
        }
    }
    let mut meta = cli_manager::load_meta();
    meta.quick_actions_enabled = Some(enabled);
    meta.quick_actions_shortcut = Some(shortcut.to_string());
    if let Err(e) = cli_manager::save_meta(&meta) {
        if next != old {
            if let Some(next) = next {
                let _ = app.global_shortcut().unregister(next);
            }
            if let Some(old) = old {
                let _ = app.global_shortcut().register(old);
            }
        }
        return Err(e);
    }
    registration.shortcut = next;
    registration.error = None;
    Ok(())
}

pub fn setup(app: &tauri::AppHandle) {
    let meta = cli_manager::load_meta();
    if let Err(e) = configure(
        app,
        meta.quick_actions_enabled.unwrap_or(true),
        meta.quick_actions_shortcut
            .as_deref()
            .unwrap_or(default_shortcut()),
    ) {
        log::warn!("Quick Actions shortcut: {e}");
        if let Ok(mut state) = app.state::<QuickActionsState>().0.lock() {
            state.error = Some(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shortcut_uses_the_platform_modifier() {
        let expected = if cfg!(target_os = "macos") {
            "Command+Shift+K"
        } else {
            "Control+Shift+K"
        };
        assert_eq!(default_shortcut(), expected);
        assert!(default_shortcut().parse::<Shortcut>().is_ok());
        assert!("not-a-shortcut".parse::<Shortcut>().is_err());
    }

    #[test]
    fn existing_meta_defaults_to_enabled_without_overwriting_other_settings() {
        let meta = serde_json::from_str::<crate::models::app_state::CliMeta>(
            r#"{"preferredPort":19421,"autoSync":true}"#,
        )
        .unwrap_or_default();
        assert!(meta.quick_actions_enabled.unwrap_or(true));
        assert!(meta.quick_actions_shortcut.is_none());
        assert_eq!(meta.preferred_port, Some(19421));
        assert_eq!(meta.auto_sync, Some(true));
    }
}
