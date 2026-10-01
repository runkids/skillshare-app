# Backend Architecture

How the Rust backend in `src-tauri/` fits together.

## Process model

- `lib.rs:run` registers the plugins, the managed state (`ServerManager`, `UpdateState`, `AutoSyncState`, `SourceHealthState`) and the commands. `setup` then:
  - builds the window, the tray and the macOS menu;
  - spawns `update_watch::spawn_background`, `auto_sync::refresh` and `source_health::spawn_background`;
  - loads the login-shell PATH;
  - runs `auto_start_server`.
- `auto_start_server` runs only after onboarding (`CliMeta.version` is set and a project exists). It syncs the Global project path from `skillshare status --json`, then starts the server for the active project.
- The server runs as `skillshare ui [-p] --port N --no-open` in the project dir. Its health check is `GET http://localhost:{port}/api/overview`. The frontend shows the server's UI in an iframe (`src/desktop/components/CliWebView.tsx`).
- `tauri.conf.json` sets `"create": false` on the main window; `main_window.rs:build` creates it from that config:
  - New windows: http/https/mailto links open in the browser; anything else is blocked.
  - Navigation: web links to non-local hosts open in the browser. `localhost`, `127.0.0.1`, `[::1]` and `tauri.localhost` stay in the window.
  - Downloads: the default destination is kept, with a notification when done.
- Closing the window hides it unless `APP_QUITTING` is set. Only the tray's Quit sets it; Quit stops the server, then calls `exit(0)`. `RunEvent::ExitRequested` also stops the server. On macOS, `RunEvent::Reopen` shows the window.
- Tray menu: Quick Sync, the source actions from `source_health` (only those with work), Open, the active project label (disabled, `TrayProjectItem`), Check for Updates…, Quit. Left-click shows the window.
- On macOS, "Check for Updates…" is also inserted under About. `app.on_menu_event` handles it from both menus and emits `check-for-updates`.

## Backend modules

- `commands/cli.rs`: detect, version, download, upgrade, install or cancel the CLI; `run_cli`; link the CLI for terminal use.
- `commands/project.rs`: project CRUD and switch. Each change refreshes the tray label and the watcher.
- `commands/server.rs`: start, stop, health check and port.
- `commands/app.rs`: app state, settings in `CliMeta`, updates, logs folder, diagnostics, `reset_all_data`.
- `commands/terminal.rs`: `get_pty_env`.
- `services/cli_manager.rs`: CLI discovery (`which`/`where`, then `%LOCALAPPDATA%\Programs\skillshare` on Windows, then app `bin/`), `exec`, version cache, installers, release download, terminal symlink, `CliMeta` load and save.
- `services/server_manager.rs`: the server supervisor.
- `services/auto_sync.rs`: the source watcher and auto-sync.
- `services/update_watch.rs`: app, CLI and skill update checks.
- `services/source_health.rs`: target drift and the source's git remote state; tray collect/push/pull.
- `commands/source_health.rs`: `get_source_health`.
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

- `auto_sync.rs`
  - `refresh` aborts the watcher task and spawns a new one. Call it at startup, after `set_auto_sync`, project add/remove/switch and reset.
  - The source dir comes from `skillshare status --json`; `notify` watches it recursively.
  - `is_source_change` ignores access events, `.git/` and editor or OS temp files.
  - `Scheduler` waits for 2s of quiet (`SETTLE`), never overlaps runs, and folds changes made during a run into one follow-up.
  - Each run calls `handle_quick_sync` only if `auto_sync` is on (default off), then always calls `update_watch::refresh_skills` and `source_health::refresh(app, false)`.
- `update_watch.rs`
  - `spawn_background` checks at launch, then wakes hourly and checks once 24h have passed since `last_update_check`. Only due checks may notify.
  - `check` gets the latest CLI (GitHub, `is_newer`), the latest app (updater) and skill updates (`skillshare check --json`, 60s timeout). A failed skill check keeps the last list.
  - `check` stores the result in `UpdateState` and emits `updates-available`. It notifies only if `notify` is set, `notify_update` is on, and the window is unfocused.
  - `unannounced` announces each app or CLI version once (`notified_*_version`), and announces skills when a name is new to `notified_skill_updates`.
  - `refresh_skills` re-runs only the skill check and emits only on change. `forget_cleared` drops names that no longer have an update and never adds any.
  - `check_updates_now` calls `check(app, false)`, so it never notifies.
- `source_health.rs`
  - `spawn_background` runs `refresh(app, true)` at launch, then every 15 min. `CHECK_LOCK` serializes runs; only a change emits `source-health` and updates the tray.
  - `skillshare diff --json [--project|--global]` (30s timeout): skills with reason `local only` are collectable; a target with any `is_sync` item is out of sync.
  - Global mode only, since `push`/`pull` use the global config: `git status --porcelain=v2 --branch` in the source dir counts uncommitted paths and ahead/behind. A `fetch` (15s, `GIT_TERMINAL_PROMPT=0`) runs only on the timed check; a failed fetch keeps the last remote state.
  - Tray items are inserted after Quick Sync: "Collect N Local Skills…" (confirm dialog, then `collect --all --json`), "Push N Changes" (`push`, default message), "Pull N Updates" (`pull`). Each runs with a 120s timeout and notifies like Quick Sync.
  - Notifies only when unfocused and `notify_update` is on, for keys new to `notified_source_health` (`local:<skill>`, `behind:<n>`). Drift and unpushed work are badge-only.
- Diagnostics: `commands/app.rs:export_diagnostics` writes `skillshare-app-diagnostics-<stamp>.txt` to Downloads, reveals it, and returns the redacted path. It includes the last 200 lines of each log.

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
  - update tracking: `last_update_check`, `notified_cli_version`, `notified_app_version`, `notified_skill_updates`, `notified_source_health`.
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
| `check-for-updates` | `update_watch.rs:CHECK_REQUESTED_EVENT` (emitted in `lib.rs`) | none |
| `sync-completed` | `lib.rs:SYNC_COMPLETED_EVENT` | none |
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
- `upgrade_cli` stops the server before `upgrade --force` because Windows cannot replace a running exe, and it restarts the server even if the upgrade fails.
- The unix installer targets `~/.local/bin`, because `/usr/local/bin` needs sudo, which the app cannot prompt for.
