# Background Services

The app's long-running watchers and checks: auto-sync, update checks, the operation log watcher, source health and the post-change audit. Process model, state files and events are in `architecture`.

## Background services

- `auto_sync.rs`
  - `refresh` aborts the watcher task and spawns a new one. Call it at startup, after `set_auto_sync`, project add/remove/switch and reset.
  - The source dir comes from `skillshare status --json`; `notify` watches it recursively.
  - `is_source_change` ignores access events, `.git/` and editor or OS temp files.
  - `Scheduler` waits for 2s of quiet (`SETTLE`), never overlaps runs, and folds changes made during a run into one follow-up.
  - Each run calls `handle_quick_sync` and `audit::run` only if `auto_sync` is on (default off), then always calls `update_watch::refresh_skills` and `source_health::refresh(app, false)`.
- `oplog_watch.rs`
  - Every CLI command, including the web UI's, appends a JSONL entry to `operations.log`: `<project>/.skillshare/logs/` (or `skillshare/`) in project mode, else `$XDG_STATE_HOME/skillshare/logs/` (`%AppData%\skillshare\logs` on Windows, `~/.local/state/skillshare/logs`). `refresh` (startup, project add/remove/switch, reset) watches the active project's log dir non-recursively; a missing dir is retried every 5s and never created, since the CLI adds the project `.gitignore` entry when it creates it. A log that appears later is read from its first line, so an `upgrade` logged meanwhile still restarts the server.
  - The tray last-sync time is seeded from successful `sync`/`sync agents` entries at watcher startup and updated from appended entries; dry runs and non-`ok` results do not count. App syncs also record their completion, per project, in `CliMeta.last_successful_sync`.
  - Only appended complete lines are parsed. A log that shrank was trimmed by the CLI right after an append, so only its last line counts.
  - `is_mutating` is an allowlist (`MUTATING`, plus `plugin`/`mcp`/`hooks` with a `MUTATING_VERBS` verb); dry runs never count. The badge checks log `check`, `diff` and `status`, so reacting to those would loop.
  - A successful `upgrade` (`is_upgrade`) first calls `ServerManager::restart_if_upgraded`, then runs `update_watch::check(app, false)` to re-read the CLI version.
  - After 1.5s of quiet (`SETTLE`) it awaits `refresh_badges` (`update_watch::refresh_skills`, then `source_health::refresh(app, false)`) inline, so entries logged meanwhile coalesce into one follow-up.
  - The tray's Quick Sync calls `refresh_badges`, Update All is followed by `source_health::refresh`, and the tray source actions also call `refresh_skills`. The title bar status panel's Check now (`check_status_now`) runs `update_watch::check(app, false)`, `source_health::refresh(app, true)` and `audit::run` together.
- `update_policy.rs` reads the CLI-owned global `updates.auto_check` and `SKILLSHARE_AUTO_UPDATE_CHECK` override through `updates --json` (10s timeout). `set_update_policy` writes through `updates on|off --json`, never rewrites YAML itself, and refuses an environment-owned preference. Unsupported old CLIs keep the previous enabled default (with environment override support); their switch is disabled until upgraded. Read failures skip automatic version checks. Refresh emits `update-policy-changed` and clears cached version badges when disabled. Resource/source checks are unaffected.
- `update_watch.rs`
  - `spawn_background` checks at launch, then wakes each minute to refresh the shared release-check preference and checks once 24h have passed since `last_update_check`. Only due checks may notify.
  - `check` gets the latest CLI (GitHub, `is_newer`), the latest app (updater), skills/repositories (`check --json`), agents (`check agents --json`, array or null), and managed plugins (`plugin check --json`, `update-available` changes deduplicated by package). Each CLI check has a 60s timeout; failures keep that kind's last list. Checks use the active project scope and serialize publication.
  - `check` stores the result in `UpdateState` and emits `updates-available`. It notifies only if `notify` is set, `notify_update` is on, and the window is unfocused.
  - `unannounced` announces each app or CLI version once (`notified_*_version`), and announces a combined resource summary when a name is new to its per-kind `notified_*_updates` list.
  - `refresh_skills` re-runs all resource checks and emits only on change. `forget_cleared` drops resolved names per kind and never adds any.
  - Automatic app/CLI checks are skipped while the shared policy is off, including the React updater and Settings entry checks; results are rechecked against the preference before automatic publication. Version notifications and badges remain hidden even after a manual check. Manual menu/Check actions bypass the preference.
  - `check_versions` (command `check_versions_now`, optional `automatic` flag) checks only the app and CLI versions: it skips the resource checks, never notifies and leaves `last_update_check` alone. `check_resources_now` runs `refresh_skills`. `refresh_skills` writes back only the resource lists, so it never undoes a version check that finished meanwhile.
- `update_all.rs` updates skills/repositories with `update --all --json`, agents with `update agents --all --json`, and each plugin with `plugin update <name> --json` (noninteractive). It holds `SYNC_LOCK`, uses 120s timeouts and `exec`'s `kill_on_drop`, then syncs skills and any updated agents, emits `sync-completed`, and refreshes resources. It never forces an update or changes extras/MCP/hooks. Notifications respect `notify_sync`; skipped updates or failed final checks report incomplete rather than success. App/CLI upgrades remain in settings.
- `source_health.rs`
  - `spawn_background` runs `refresh(app, true)` at launch, then every 15 min. `CHECK_LOCK` serializes runs; only a change emits `source-health` and updates tray status lines/actions in place.
  - Project add/remove/switch call `project_changed`: it clears the state (badge and tray items go away), emits, then runs `refresh(app, false)`. `refresh` drops a result whose project is no longer active (`is_news`), so a slow check for the old project never overwrites the new one.
  - `skillshare diff --json [--project|--global]` (30s timeout): a target with any `is_sync` item is out of sync.
  - Global mode only, since `push`/`pull` use the global config: `git status --porcelain=v2 --branch` in the source dir counts uncommitted paths and ahead/behind. A `fetch` (15s, `GIT_TERMINAL_PROMPT=0`) runs only on the timed check; a failed fetch keeps the last remote state.
  - Tray actions are grouped after Update All: "Push N Changes" (`push`, default message), "Pull N Updates" (`pull`). Each runs with a 120s timeout and notifies like Quick Sync.
  - Notifies only when unfocused and `notify_update` is on, for keys new to `notified_source_health` (`local:<skill>`, `behind:<n>`). Drift and unpushed work are badge-only.
- `audit.rs`
  - `run` is called at the end of Update All, a successful tray Pull, and an auto-sync run. It only reports; nothing is blocked or rolled back.
  - It runs `audit --format json --yes` and `audit agents --format json --yes` with the active mode (60s timeout each, `kill_on_drop`) over the whole source: Pull and auto-sync do not know which skills changed. `audit` exits non-zero at its block threshold, so stdout is parsed regardless of the exit code; a failed audit keeps the last findings.
  - It stores LOW to CRITICAL findings (INFO dropped) in `AuditState`, most severe first, and emits `audit-report`; the tray shows only the HIGH/CRITICAL count as "N security issues". `AUDIT_LOCK` serializes runs; `project_changed` (project add/remove/switch) waits for it, then clears and emits.
  - Notifies, if `notify_sync` is on, for HIGH/CRITICAL keys (`<kind>:<skill>:<fingerprint>`) new to `notified_audit_findings`, which is replaced by the current keys. LOW/MEDIUM are badge-only.
- Diagnostics: `commands/app.rs:export_diagnostics` writes `skillshare-app-diagnostics-<stamp>.txt` to Downloads, reveals it, and returns the redacted path. It includes the last 200 lines of each log.
