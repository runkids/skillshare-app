//! Cached, grouped tray menu. Opening it never runs a CLI command or replaces the menu.

use crate::services::{
    self, audit::AuditFinding, cli_manager, project_store, source_health::SourceHealth,
};
use chrono::{DateTime, Utc};
use std::sync::{atomic::Ordering, Mutex};
use std::time::Duration;
use tauri::menu::{
    CheckMenuItem, CheckMenuItemBuilder, Menu, MenuBuilder, MenuItem, MenuItemBuilder, Submenu,
};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_opener::OpenerExt;

pub const PROJECT_REQUESTED_EVENT: &str = "tray-project-requested";
pub const AUTO_SYNC_CHANGED_EVENT: &str = "auto-sync-changed";
const PROJECT_PREFIX: &str = "tray_project:";

struct TrayMenu {
    menu: Menu<Wry>,
    last_sync: MenuItem<Wry>,
    update_all: MenuItem<Wry>,
    project: Submenu<Wry>,
    open_source: MenuItem<Wry>,
    auto_sync: CheckMenuItem<Wry>,
    contents: Mutex<Contents>,
}

#[derive(Default)]
struct Contents {
    updates: usize,
    health: SourceHealth,
    security: usize,
    statuses: Rows,
    actions: Rows,
    projects: Vec<CheckMenuItem<Wry>>,
}

/// Keep existing handles and mutate only changed labels or membership.
#[derive(Default)]
struct Rows(Vec<(MenuItem<Wry>, String)>);

impl Rows {
    fn update(
        &mut self,
        app: &AppHandle,
        menu: &Menu<Wry>,
        anchor: &str,
        labels: Vec<(&str, String)>,
    ) -> tauri::Result<()> {
        let mut kept = Vec::new();
        for (item, text) in self.0.drain(..) {
            if labels.iter().any(|(id, _)| item.id() == *id) {
                kept.push((item, text));
            } else {
                menu.remove(&item)?;
            }
        }
        self.0 = kept;
        let base = menu
            .items()?
            .iter()
            .position(|i| i.id() == anchor)
            .unwrap_or(0)
            + 1;
        for (offset, (id, text)) in labels.into_iter().enumerate() {
            if let Some((item, before)) = self.0.iter_mut().find(|(i, _)| i.id() == id) {
                if *before != text {
                    item.set_text(&text)?;
                    *before = text;
                }
            } else {
                let item = MenuItemBuilder::with_id(id, &text)
                    .enabled(anchor != "last_sync")
                    .build(app)?;
                menu.insert(&item, base + offset)?;
                self.0.push((item, text));
            }
        }
        Ok(())
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

fn status_labels(
    updates: usize,
    health: &SourceHealth,
    security: usize,
) -> Vec<(&'static str, String)> {
    let mut labels = Vec::new();
    if updates > 0 {
        labels.push((
            "status_updates",
            format!("{updates} update{} available", plural(updates)),
        ));
    }
    let drift = health.out_of_sync_targets.len();
    if drift > 0 {
        labels.push((
            "status_drift",
            format!("{drift} target{} out of sync", plural(drift)),
        ));
    }
    if let Some(git) = &health.git {
        let n = git.uncommitted + git.ahead;
        if n > 0 {
            labels.push((
                "status_unpushed",
                format!("{n} unpushed change{}", plural(n)),
            ));
        }
    }
    if security > 0 {
        labels.push((
            "status_security",
            format!("{security} security issue{}", plural(security)),
        ));
    }
    labels
}

fn action_labels(health: &SourceHealth) -> Vec<(&'static str, String)> {
    let mut labels = Vec::new();
    let n = health.local_skills.len();
    if n > 0 {
        labels.push((
            "source_collect",
            format!("Collect {n} Local Skill{}…", plural(n)),
        ));
    }
    if let Some(git) = &health.git {
        let n = git.uncommitted + git.ahead;
        if n > 0 {
            labels.push(("source_push", format!("Push {n} Change{}", plural(n))));
        }
        if git.behind > 0 {
            labels.push((
                "source_pull",
                format!("Pull {} Update{}", git.behind, plural(git.behind)),
            ));
        }
    }
    labels
}

fn last_sync_label(timestamp: Option<&str>, now: DateTime<Utc>) -> String {
    let Some(time) = timestamp.and_then(|t| DateTime::parse_from_rfc3339(t).ok()) else {
        return "Last sync: Never".into();
    };
    let seconds = (now - time.with_timezone(&Utc)).num_seconds().max(0);
    let age = match seconds {
        0..60 => "just now".into(),
        60..3600 => format!("{} min ago", seconds / 60),
        3600..86400 => format!("{} hr ago", seconds / 3600),
        _ => format!(
            "{} day{} ago",
            seconds / 86400,
            plural((seconds / 86400) as usize)
        ),
    };
    format!("Last sync: {age}")
}

fn refresh_statuses(
    app: &AppHandle,
    tray: &TrayMenu,
    contents: &mut Contents,
) -> tauri::Result<()> {
    let labels = status_labels(contents.updates, &contents.health, contents.security);
    contents
        .statuses
        .update(app, &tray.menu, "last_sync", labels)
}

fn report(result: tauri::Result<()>) {
    if let Err(e) = result {
        log::warn!("Could not update tray menu: {e}");
    }
}

// Native setters dispatch to the main thread. Serialize whole updates there too,
// so a click cannot wait on a mutex held by a worker waiting for that same thread.
fn with_tray(app: &AppHandle, update: impl FnOnce(&AppHandle, &TrayMenu) + Send + 'static) {
    let handle = app.clone();
    report(app.run_on_main_thread(move || {
        if let Some(tray) = handle.try_state::<TrayMenu>() {
            update(&handle, &tray);
        }
    }));
}

pub fn refresh_update_count(app: &AppHandle, count: usize) {
    with_tray(app, move |app, tray| {
        let mut contents = tray.contents.lock().unwrap_or_else(|e| e.into_inner());
        if contents.updates != count {
            report(tray.update_all.set_text(format!("Update All ({count})")));
            contents.updates = count;
            report(refresh_statuses(app, tray, &mut contents));
        }
        report(
            tray.update_all
                .set_enabled(count > 0 && !services::update_all::is_running()),
        );
    });
}

pub fn refresh_source_health(app: &AppHandle, health: &SourceHealth) {
    let health = health.clone();
    with_tray(app, move |app, tray| {
        let mut contents = tray.contents.lock().unwrap_or_else(|e| e.into_inner());
        contents.health = health.clone();
        report(refresh_statuses(app, tray, &mut contents));
        report(
            contents
                .actions
                .update(app, &tray.menu, "update_all", action_labels(&health)),
        );
    });
}

pub fn refresh_audit(app: &AppHandle, findings: &[AuditFinding]) {
    let security = findings
        .iter()
        .filter(|f| matches!(f.severity.as_str(), "HIGH" | "CRITICAL"))
        .count();
    with_tray(app, move |app, tray| {
        let mut contents = tray.contents.lock().unwrap_or_else(|e| e.into_inner());
        contents.security = security;
        report(refresh_statuses(app, tray, &mut contents));
    });
}

pub fn refresh_auto_sync(app: &AppHandle) {
    let enabled = cli_manager::load_meta().auto_sync.unwrap_or(false);
    with_tray(app, move |_, tray| {
        report(tray.auto_sync.set_checked(enabled))
    });
    let _ = app.emit(AUTO_SYNC_CHANGED_EVENT, enabled);
}

fn refresh_last_sync(app: &AppHandle) {
    with_tray(app, |_, tray| {
        let store = project_store::load();
        let meta = cli_manager::load_meta();
        let time = store
            .active_project_id
            .as_ref()
            .and_then(|id| meta.last_successful_sync.get(id));
        let label = last_sync_label(time.map(String::as_str), Utc::now());
        if tray.last_sync.text().ok().as_ref() != Some(&label) {
            report(tray.last_sync.set_text(label));
        }
    });
}

pub fn record_sync(app: &AppHandle, project_id: Option<&str>, time: DateTime<Utc>) {
    let Some(id) = project_id else {
        return;
    };
    let mut meta = cli_manager::load_meta();
    let newer = meta
        .last_successful_sync
        .get(id)
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .is_none_or(|before| time > before);
    if newer {
        meta.last_successful_sync
            .insert(id.to_string(), time.to_rfc3339());
        if let Err(e) = cli_manager::save_meta(&meta) {
            log::warn!("Could not save last successful sync: {e}");
        }
        refresh_last_sync(app);
    }
}

pub fn refresh_projects(app: &AppHandle) {
    with_tray(app, |app, tray| {
        let store = project_store::load();
        let name = store
            .active_project()
            .map(|p| p.name.as_str())
            .unwrap_or("No active project");
        let label = format!("Project: {name}");
        if tray.project.text().ok().as_ref() != Some(&label) {
            report(tray.project.set_text(label));
            // Linux's AppIndicator keeps the submenu heading until its membership changes.
            #[cfg(target_os = "linux")]
            report((|| -> tauri::Result<()> {
                if let Some(position) = tray
                    .menu
                    .items()?
                    .iter()
                    .position(|i| i.id() == tray.project.id())
                {
                    tray.menu.remove(&tray.project)?;
                    tray.menu.insert(&tray.project, position)?;
                }
                Ok(())
            })());
        }
        report(tray.project.set_enabled(!store.projects.is_empty()));
        report(
            tray.open_source
                .set_enabled(store.active_project().is_some()),
        );
        let mut contents = tray.contents.lock().unwrap_or_else(|e| e.into_inner());
        report((|| -> tauri::Result<()> {
            // Switches only change checkmarks. Add/remove rebuilds the affected submenu.
            let ids: Vec<_> = store
                .projects
                .iter()
                .map(|p| format!("{PROJECT_PREFIX}{}", p.id))
                .collect();
            if contents
                .projects
                .iter()
                .map(|i| i.id().as_ref())
                .ne(ids.iter().map(String::as_str))
            {
                for item in contents.projects.drain(..) {
                    tray.project.remove(&item)?;
                }
                for (project, id) in store.projects.iter().zip(&ids) {
                    let item = CheckMenuItemBuilder::with_id(id, &project.name).build(app)?;
                    tray.project.append(&item)?;
                    contents.projects.push(item);
                }
            }
            for (project, item) in store.projects.iter().zip(&contents.projects) {
                item.set_checked(store.active_project_id.as_deref() == Some(project.id.as_str()))?;
            }
            Ok(())
        })());
    });
    refresh_last_sync(app);
}

async fn reveal_source(app: &AppHandle) -> Result<(), String> {
    let store = project_store::load();
    let project = store.active_project().ok_or("Set up a project first")?;
    let mode = if project.project_type == crate::models::project::ProjectType::Project {
        "--project"
    } else {
        "--global"
    };
    let cli = tokio::time::timeout(Duration::from_secs(10), cli_manager::detect_cli())
        .await
        .map_err(|_| "CLI detection timed out")?
        .ok_or("CLI not found")?;
    let args = ["status", "--json", mode].map(String::from);
    let output = tokio::time::timeout(
        Duration::from_secs(15),
        cli_manager::exec(&cli, &args, Some(&project.path)),
    )
    .await
    .map_err(|_| "Source lookup timed out")??;
    let status: serde_json::Value = serde_json::from_str(&output).map_err(|e| e.to_string())?;
    let path = status["source"]["path"]
        .as_str()
        .ok_or("CLI status is missing source.path")?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| e.to_string())
}

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let last_sync = MenuItemBuilder::with_id("last_sync", "Last sync: Never")
        .enabled(false)
        .build(app)?;
    let quick_sync = MenuItemBuilder::with_id("quick_sync", "Quick Sync").build(app)?;
    let update_all = MenuItemBuilder::with_id("update_all", "Update All (0)")
        .enabled(false)
        .build(app)?;
    let quick_actions = MenuItemBuilder::with_id("quick_actions", "Quick Actions…").build(app)?;
    let project = Submenu::new(app, "Project: No active project", false)?;
    let open_source = MenuItemBuilder::with_id("open_source", "Open Source Folder")
        .enabled(false)
        .build(app)?;
    let auto_sync = CheckMenuItemBuilder::with_id("auto_sync", "Auto Sync")
        .checked(cli_manager::load_meta().auto_sync.unwrap_or(false))
        .build(app)?;
    let open_app = MenuItemBuilder::with_id("open_app", "Open Skillshare App").build(app)?;
    let check_updates = crate::check_updates_item(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&last_sync)
        .separator()
        .item(&quick_sync)
        .item(&update_all)
        .item(&quick_actions)
        .separator()
        .item(&project)
        .item(&open_source)
        .separator()
        .item(&auto_sync)
        .separator()
        .item(&open_app)
        .item(&check_updates)
        .item(&quit)
        .build()?;
    app.manage(TrayMenu {
        menu: menu.clone(),
        last_sync,
        update_all,
        project,
        open_source,
        auto_sync,
        contents: Mutex::new(Contents::default()),
    });
    refresh_projects(app.handle());

    let mut tray = TrayIconBuilder::new();
    // macOS menu bar icons are monochrome templates tinted by the system.
    #[cfg(target_os = "macos")]
    let template_icon =
        tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))
            .inspect_err(|e| log::warn!("Failed to load tray template icon: {e}"))
            .ok();
    #[cfg(not(target_os = "macos"))]
    let template_icon: Option<tauri::image::Image<'static>> = None;
    if let Some(icon) = template_icon {
        tray = tray.icon(icon).icon_as_template(true);
    } else if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.tooltip("Skillshare App")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quick_sync" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = crate::handle_quick_sync(&app, true).await;
                    services::oplog_watch::refresh_badges(&app).await;
                });
            }
            "update_all" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    services::update_all::run(&app).await;
                    services::source_health::refresh(&app, false).await;
                });
            }
            "quick_actions" => services::quick_actions::request_open(app),
            "open_app" => crate::show_main_window(app),
            "auto_sync" => {
                let enabled = !cli_manager::load_meta().auto_sync.unwrap_or(false);
                if let Err(e) = crate::commands::app::set_auto_sync(app.clone(), enabled) {
                    log::warn!("Could not change auto-sync: {e}");
                    refresh_auto_sync(app);
                }
            }
            "open_source" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = reveal_source(&app).await {
                        log::warn!("Could not reveal skills source: {e}");
                    }
                });
            }
            "quit" => {
                crate::APP_QUITTING.store(true, Ordering::SeqCst);
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = app
                        .state::<services::server_manager::ServerManager>()
                        .stop()
                        .await;
                    app.exit(0);
                });
            }
            id => {
                if let Some(id) = id.strip_prefix(PROJECT_PREFIX) {
                    // Restore exclusive checkmarks before the shared switcher persists the change.
                    refresh_projects(app);
                    let _ = app.emit(PROJECT_REQUESTED_EVENT, id);
                } else {
                    services::source_health::on_menu_event(app, id);
                }
            }
        })
        .build(app)?;

    let app = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            refresh_last_sync(&app);
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use services::source_health::GitState;

    #[test]
    fn absent_counts_do_not_add_status_or_actions() {
        assert!(status_labels(0, &SourceHealth::default(), 0).is_empty());
        assert!(action_labels(&SourceHealth::default()).is_empty());
    }

    #[test]
    fn groups_use_plain_counts_and_preserve_source_actions() {
        let health = SourceHealth {
            local_skills: vec!["a".into(), "b".into()],
            out_of_sync_targets: vec!["claude".into()],
            git: Some(GitState {
                uncommitted: 1,
                ahead: 1,
                behind: 1,
            }),
        };
        assert_eq!(
            status_labels(3, &health, 2),
            vec![
                ("status_updates", "3 updates available".into()),
                ("status_drift", "1 target out of sync".into()),
                ("status_unpushed", "2 unpushed changes".into()),
                ("status_security", "2 security issues".into()),
            ]
        );
        assert_eq!(
            action_labels(&health),
            vec![
                ("source_collect", "Collect 2 Local Skills…".into()),
                ("source_push", "Push 2 Changes".into()),
                ("source_pull", "Pull 1 Update".into()),
            ]
        );
    }

    #[test]
    fn last_sync_handles_missing_invalid_future_and_age_boundaries() {
        let Some(now) = DateTime::parse_from_rfc3339("2026-10-02T12:00:00Z")
            .map(|t| t.with_timezone(&Utc))
            .ok()
        else {
            panic!("invalid fixture");
        };
        assert_eq!(last_sync_label(None, now), "Last sync: Never");
        assert_eq!(last_sync_label(Some("invalid"), now), "Last sync: Never");
        assert_eq!(
            last_sync_label(Some("2026-10-02T12:01:00Z"), now),
            "Last sync: just now"
        );
        assert_eq!(
            last_sync_label(Some("2026-10-02T11:55:00Z"), now),
            "Last sync: 5 min ago"
        );
        assert_eq!(
            last_sync_label(Some("2026-10-02T11:00:00Z"), now),
            "Last sync: 1 hr ago"
        );
        assert_eq!(
            last_sync_label(Some("2026-10-01T12:00:00Z"), now),
            "Last sync: 1 day ago"
        );
    }
}
