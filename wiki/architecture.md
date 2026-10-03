# Backend Architecture

How the Rust backend in `src-tauri/` fits together.

## Process model

- `lib.rs:run` registers the plugins, the managed state (`ServerManager`, `UpdateState`, `AutoSyncState`, `OplogWatchState`, `SourceHealthState`, `AuditState`) and the commands. `setup` then:
  - builds the window, the tray and the macOS menu;
  - spawns `update_watch::spawn_background`, `auto_sync::refresh`, `oplog_watch::refresh` and `source_health::spawn_background`;
  - loads the login-shell PATH;
  - runs `auto_start_server`.
- `auto_start_server` runs only after onboarding (`CliMeta.version` is set and a project exists). It syncs the Global project path from `skillshare status --json`, then starts the server for the active project.
- The server runs as `skillshare ui [-p] --port N --no-open` in the project dir. Its health check is `GET http://localhost:{port}/api/overview`. Only a server this app started counts as running: another server on the port, such as one an earlier app instance left behind, may serve a Web UI whose files are gone. The frontend shows the server's UI in an iframe (`src/desktop/components/CliWebView.tsx`).
- `tauri.conf.json` sets `"create": false` on the main window; `main_window.rs:build` creates it from that config:
  - New windows: http/https/mailto links open in the browser; anything else is blocked.
  - Navigation: web links to non-local hosts open in the browser. `localhost`, `127.0.0.1`, `[::1]` and `tauri.localhost` stay in the window.
  - Downloads: the default destination is kept, with a notification when done.
- Closing the window hides it unless `APP_QUITTING` is set. Only the tray's Quit sets it; Quit stops the server, then calls `exit(0)`. `RunEvent::ExitRequested` also stops the server. On macOS, `RunEvent::Reopen` shows the window.
- `tray.rs` owns the grouped menu: disabled last-sync and nonzero resource-update/drift/unpushed/HIGH-CRITICAL counts; Quick Sync, Update All (N), source actions; a checked project submenu and Open Source Folder; Auto Sync; Open, Check for Updates…, Quit. Existing handles update in place; only membership changes insert/remove items (Linux also reinserts a renamed project submenu to refresh its AppIndicator heading). Left-click opens the menu, like right-click. Project requests use `ProjectContext.switchWithRestart`, the same flow as the in-app switcher; source-folder lookup uses scoped `status --json` on click (15s timeout).
- On macOS, "Check for Updates…" is also inserted under About. `app.on_menu_event` handles it from both menus and emits `check-for-updates`.

## Backend modules

- `commands/cli.rs`: detect, version, download, upgrade, install or cancel the CLI; `run_cli`; link the CLI for terminal use.
- `commands/project.rs`: project CRUD and switch. Each change refreshes the tray project submenu, both watchers and source health.
- `commands/server.rs`: start, stop, health check and port.
- `commands/app.rs`: app state, settings in `CliMeta`, updates, logs folder, diagnostics, `reset_all_data`.
- `commands/terminal.rs`: `get_pty_env`.
- `commands/activity.rs`: `get_activity` runs `skillshare log --json --since 7d --tail 200` for the active project (15s timeout) and returns entries newest first, dropping successful `check` runs.
- `services/cli_manager.rs`: CLI discovery (`which`/`where`, then `%LOCALAPPDATA%\Programs\skillshare` on Windows, then app `bin/`), `exec`, version cache, installers, release download, terminal symlink, `CliMeta` load and save.
- `services/server_manager.rs`: the server supervisor.
- `services/auto_sync.rs`: the source watcher and auto-sync.
- `services/oplog_watch.rs`: the CLI operation log watcher that keeps the badges live.
- `services/update_watch.rs`: app, CLI and per-kind resource update checks; `services/update_all.rs`: tray resource updates and sync.
- `services/source_health.rs`: target drift and the source's git remote state; tray push/pull.
- `commands/source_health.rs`: `get_source_health`.
- `services/audit.rs`: the security audit after changes; `commands/audit.rs`: `get_audit_report`.
- `commands/status.rs`: `check_status_now`, the title bar status panel's Check now; it re-runs the update, source health and audit checks.
- `services/diagnostics.rs`: the report text, with home redacted to `~`.
- `services/project_store.rs`: `projects.json`, the active project, `active_project_mode`.
- `utils/env.rs`: the login-shell PATH and the child-process environment.
- `utils/paths.rs`: `app_data_dir`, `logs_dir`.
- `utils/fs.rs`: `write_atomic` (temp file, fsync, rename).
- `models/app_state.rs`: `CliMeta`, `OnboardingStatus`, `AppInfo`.
- `models/project.rs`: `Project`, `ProjectType`, `ProjectStore`.

## Server supervisor

- `start` takes the `lifecycle` mutex and calls `end_generation`, which bumps `generation` and kills the process. It then clears `exits`, runs `launch`, and spawns `supervise`.
- `stop` takes `lifecycle` and calls `end_generation`.
- `launch` runs these steps in order:
  1. Kill the tracked child.
  2. Probe ports from `preferred_port` (default 19420) to +10 by TCP connect.
  3. Kill an orphaned server.
  4. Spawn with `build_env_for_child`, sending output to `server.log` (truncated each start).
  5. Write the pidfile.
  6. Poll the health check every 500 ms, 20 tries. On failure, the error includes the last 20 lines of `server.log`.
- `supervise` polls `try_wait` every 1s. After an unexpected exit, it waits `RESTART_DELAYS` (1s, 2s, 5s), counting exits within `RESTART_WINDOW` (60s).
  - When exits outrun the delays, it emits `server-stopped` and gives up.
  - A restart emits `server-restarted` with the port, which may have changed.
- `server.pid` holds `"<pid> <port>"`. `kill_orphaned_server` deletes it, then sends `kill -TERM` only if `lsof -t -a -iTCP:<port> -sTCP:LISTEN -c /^skillshare$/` lists that pid and the pid is not the app's own. A user-started `skillshare ui` is never killed.

## Background services

Auto-sync, update checks, the operation log watcher, source health and the audit are described in `wiki/background-services.md` (topic `background-services`).

## State and files

- The app data dir is `dirs::data_dir()` joined with `com.skillshare.app-dev` (debug builds) or `com.skillshare.app` (release builds). Inside it:
  - `cli-meta.json`: `CliMeta` in camelCase, written with `write_atomic`. An unreadable file loads as the default.
  - `projects.json`: `ProjectStore` (`projects`, `active_project_id`).
  - `bin/`: the app's own copy of the CLI.
  - `server.pid`.
  - `logs/app.log` (Info level, 1 MB, one rotated file) and `logs/server.log`.
- `CliMeta` fields:
  - install: `version`, `path`, `source`, `installed_at`, `binary_modified_ms`;
  - settings: `preferred_port`, `notify_sync`, `notify_update`, `auto_sync`;
  - sync tracking: `last_successful_sync` (project ID to RFC 3339 timestamp; tray age refreshes every minute);
  - update tracking: `last_update_check`, `notified_cli_version`, `notified_app_version`, `notified_skill_updates`, `notified_repository_updates`, `notified_agent_updates`, `notified_plugin_updates`, `notified_source_health`, `notified_audit_findings`.
- Project store rules:
  - A corrupt `projects.json` is moved to `projects.json.corrupt-<ts>`.
  - Duplicate paths are rejected after canonicalizing.
  - Only one Global project is allowed.
  - The first project added becomes active.
- `utils/env.rs:load_login_shell_path` reads `$SHELL`'s login PATH once into a `OnceCell`. On Windows it returns `None`, since GUI apps inherit the user's PATH there.
- `build_env_for_child` builds PATH in this order: the login PATH, the tool dirs (Volta, fnm, Homebrew, `/usr/local/bin`, Cargo, Go, `~/bin`, `~/.local/bin`), then the process PATH. Duplicates are dropped, joined with the platform separator. It also sets `HOME`, `LANG`/`LC_ALL`, `TERM`, `SSH_AUTH_SOCK` and `SHELL`.
- On Windows, `install_cli` skips this environment and spawns with `CREATE_NO_WINDOW`.

## Events and commands

| Event | Constant | Payload |
|---|---|---|
| `server-restarted` | `server_manager.rs:SERVER_RESTARTED_EVENT` | port |
| `server-stopped` | `server_manager.rs:SERVER_STOPPED_EVENT` | none |
| `updates-available` | `update_watch.rs:UPDATES_EVENT` | `AvailableUpdates` |
| `source-health` | `source_health.rs:SOURCE_HEALTH_EVENT` | `SourceHealth` |
| `audit-report` | `audit.rs:AUDIT_EVENT` | `AuditFinding[]` |
| `check-for-updates` | `update_watch.rs:CHECK_REQUESTED_EVENT` (emitted in `lib.rs`) | none |
| `sync-completed` | `lib.rs:SYNC_COMPLETED_EVENT` | none |
| `tray-project-requested` | `tray.rs:PROJECT_REQUESTED_EVENT` | project ID |
| `auto-sync-changed` | `tray.rs:AUTO_SYNC_CHANGED_EVENT` | enabled (boolean) |
| `cli-install-output` | `cli_manager.rs:INSTALL_OUTPUT_EVENT` | `{stream, line}` |

To add a command:

1. Define a `#[tauri::command]` in `src-tauri/src/commands/<area>.rs`. Return `Result<T, String>` if it can fail.
2. Register it in the `tauri::generate_handler![...]` call in `lib.rs:run`.
3. Add a wrapper to `tauriBridge` in `src/desktop/api/tauri-bridge.ts`. Pass camelCase args (`invoke('x', { cliPath })`) and mirror the serde types.

`capabilities/default.json` holds plugin permissions only; app commands need no entry.

## Gotchas

- Hold `lifecycle` across start, stop and every supervised restart, so `stop()` waits for a restart in progress and kills its new process. `supervise` re-checks `generation` after taking the lock.
- `SYNC_LOCK` (`lib.rs`) spans all of `handle_quick_sync`, so tray and auto-sync runs never overlap. Quick Sync times out after 120s; `exec` uses `kill_on_drop` so the CLI dies with it.
- `-c /^skillshare$/` is an exact match, so the app binary (`skillshare-app`) never qualifies. There is no `lsof` on Windows, so orphan cleanup does nothing there.
- A port held by the user's own `skillshare ui` fails the TCP probe and is skipped.
- `launch` awaits `load_login_shell_path`, because an app launched from Finder lacks the user's PATH (git, brew, agent CLIs).
- `get_app_state` and `check` call `refresh_cached_version`, because the CLI can be upgraded outside the app. A changed binary mtime triggers a re-read.
- `start_server` takes `project_dir` from the frontend, but takes the mode from the stored active project.
- `upgrade_cli` keeps the server running during `upgrade --force`, because the download can take minutes, then restarts it on the new binary. Only on Windows does it stop the server first, because Windows cannot replace a running exe; there it restarts the server even if the upgrade fails.
- The Web UI's "Update now" makes the CLI restart itself: the server exits with status 0 and a detached `__ui-restart` helper starts a new server on the same port. On a clean exit, `supervise` stops any `skillshare` process listening on that port (up to 5s, via `lsof`), so the relaunch keeps the port and no unsupervised server outlives the app.
- The unix installer targets `~/.local/bin`, because `/usr/local/bin` needs sudo, which the app cannot prompt for.
