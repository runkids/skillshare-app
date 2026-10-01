use crate::services::{cli_manager, project_store};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// `log` only reads local files; this guards against a CLI stuck on a lock or prompt.
const ACTIVITY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
/// How far back the timeline looks, as a `skillshare log --since` value.
const ACTIVITY_SINCE: &str = "7d";
/// The newest entries kept; `--tail` caps the operations and audit logs separately.
const MAX_ENTRIES: usize = 200;

/// One operation from the CLI's log, shaped for the Activity view.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    /// RFC 3339, with the offset the CLI wrote.
    pub ts: String,
    pub cmd: String,
    /// `ok`, `error`, `partial` or `blocked`.
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: Option<u64>,
    /// Skills or targets the operation touched.
    pub subjects: Vec<String>,
    /// A count summary such as "7 targets, 1 failed".
    pub detail: Option<String>,
}

/// A line of `skillshare log --json`.
#[derive(Deserialize)]
struct RawEntry {
    ts: String,
    cmd: String,
    #[serde(default)]
    args: Map<String, Value>,
    status: String,
    msg: Option<String>,
    ms: Option<u64>,
}

/// Recent operations in the active project, newest first. Read-only: runs `skillshare log`.
#[tauri::command]
pub async fn get_activity() -> Result<Vec<ActivityEntry>, String> {
    let cli_path = cli_manager::detect_cli()
        .await
        .ok_or("Skillshare CLI not found")?;
    let (dir, is_project) = project_store::active_project_mode(&project_store::load());
    let mode = if is_project { "--project" } else { "--global" };
    let tail = MAX_ENTRIES.to_string();
    let args = [
        "log",
        "--json",
        "--since",
        ACTIVITY_SINCE,
        "--tail",
        &tail,
        mode,
    ]
    .map(String::from);
    let stdout = tokio::time::timeout(
        ACTIVITY_TIMEOUT,
        cli_manager::exec(&cli_path, &args, dir.as_deref()),
    )
    .await
    .map_err(|_| format!("timed out after {}s", ACTIVITY_TIMEOUT.as_secs()))??;
    Ok(parse_activity(&stdout))
}

/// Parse JSONL log output into timeline entries, newest first. Skips malformed lines and
/// successful `check` runs, which the app itself triggers after every source change.
fn parse_activity(stdout: &str) -> Vec<ActivityEntry> {
    let mut entries: Vec<(DateTime<FixedOffset>, ActivityEntry)> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| match serde_json::from_str::<RawEntry>(line) {
            Ok(raw) => Some(raw),
            Err(e) => {
                log::warn!("Unexpected `skillshare log --json` line: {e}");
                None
            }
        })
        .filter(|raw| !(raw.cmd == "check" && raw.status == "ok"))
        .filter_map(|raw| {
            let at = DateTime::parse_from_rfc3339(&raw.ts).ok()?;
            Some((at, to_entry(raw)))
        })
        .collect();
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries.truncate(MAX_ENTRIES);
    entries.into_iter().map(|(_, e)| e).collect()
}

fn to_entry(raw: RawEntry) -> ActivityEntry {
    ActivityEntry {
        subjects: subjects(&raw.cmd, &raw.args),
        detail: detail(&raw.cmd, &raw.args),
        ts: raw.ts,
        cmd: raw.cmd,
        status: raw.status,
        message: raw.msg.filter(|m| !m.is_empty()),
        duration_ms: raw.ms,
    }
}

/// Skill and target names from the args each command logs.
fn subjects(cmd: &str, args: &Map<String, Value>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for key in ["installed_skills", "names", "targets"] {
        if let Some(Value::Array(items)) = args.get(key) {
            out.extend(items.iter().filter_map(Value::as_str).map(String::from));
        }
    }
    // `name` can be a flag placeholder such as "--all-stream"; `target` can be a path.
    if let Some(name) = args.get("name").and_then(Value::as_str) {
        if !name.is_empty() && !name.starts_with('-') {
            out.push(name.to_string());
        }
    }
    if let Some(target) = args.get("target").and_then(Value::as_str) {
        if !target.contains(['/', '\\']) {
            out.push(target.to_string());
        }
    }
    // A failed install names no skills; its source says what was being installed.
    if cmd == "install" && out.is_empty() {
        if let Some(source) = args.get("source").and_then(Value::as_str) {
            out.push(source.to_string());
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|s| seen.insert(s.clone()));
    out
}

/// Totals for commands that fan out over targets or skills.
fn detail(cmd: &str, args: &Map<String, Value>) -> Option<String> {
    let (total_key, failed_key, noun) = match cmd {
        "sync" => ("targets_total", "targets_failed", "target"),
        "update" => ("results_total", "results_failed", "skill"),
        _ => return None,
    };
    let total = args.get(total_key)?.as_u64()?;
    let mut text = format!("{total} {noun}{}", if total == 1 { "" } else { "s" });
    if let Some(failed) = args.get(failed_key).and_then(Value::as_u64) {
        if failed > 0 {
            text.push_str(&format!(", {failed} failed"));
        }
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from `skillshare log --json --global` (CLI 0.23): operations, then audit.
    const SAMPLE: &str = r#"{"ts":"2026-10-01T22:40:13+08:00","cmd":"push","args":{"dry_run":false,"message":"Update skills","scope":"ui"},"status":"ok","ms":3317}
{"ts":"2026-10-01T22:33:06+08:00","cmd":"check","args":{"errors":0,"repos_checked":0,"scope":"global","skills_checked":0,"updates_available":0},"status":"ok","ms":1230}
{"ts":"2026-10-01T14:11:30+08:00","cmd":"update","args":{"force":false,"name":"--all-stream","results_blocked":0,"results_failed":2,"results_total":16,"scope":"ui","skip_audit":false},"status":"partial","msg":"2 update(s) failed","ms":26406}
{"ts":"2026-10-01T13:00:48+08:00","cmd":"sync","args":{"dry_run":false,"force":false,"scope":"global","targets_failed":0,"targets_total":7},"status":"ok","ms":41}
{"ts":"2026-10-01T03:43:33+08:00","cmd":"init","status":"error","msg":"skillshare is already initialized (global config: ~/.config/skillshare/config.yaml). Run 'skillshare init --discover' to add new agents, or 'skillshare init -p' to initialize project-level skills"}
{"ts":"2026-09-30T01:20:45+08:00","cmd":"target","args":{"action":"add","name":"codex","scope":"ui","target":"~/.agents/skills"},"status":"ok","ms":7}
{"ts":"2026-09-29T23:09:20+08:00","cmd":"target-file-edit","args":{"path":"APPEND_SYSTEM.md","scope":"ui","target":"pi"},"status":"ok"}
{"ts":"2026-09-28T02:53:59+08:00","cmd":"uninstall","args":{"count":1,"force":false,"names":["devops__docker-expert"],"scope":"ui"},"status":"ok","ms":6}
{"ts":"2026-09-25T02:00:46+08:00","cmd":"install","args":{"installed_skills":["agents-context-router"],"mode":"global","skill_count":1,"source":"runkids/agents-context-router","threshold":"CRITICAL"},"status":"ok","ms":1169}
{"ts":"2026-09-24T01:05:08+08:00","cmd":"install","args":{"mode":"global","source":"runkids/skillshare","threshold":"CRITICAL"},"status":"error","msg":"failed to remove existing skill: permission denied","ms":4655}
{"ts":"2026-10-01T13:05:52+08:00","cmd":"audit","args":{"critical":0,"failed":0,"high":0,"info":1,"info_skills":["agents-context-router"],"low":0,"medium":0,"mode":"ui","name":"agents-context-router","passed":0,"risk_label":"low","risk_score":1,"scanned":1,"scope":"single","threshold":"CRITICAL","warning":1,"warning_skills":["agents-context-router"]},"status":"ok","ms":21}"#;

    fn find<'a>(entries: &'a [ActivityEntry], cmd: &str, status: &str) -> &'a ActivityEntry {
        let Some(entry) = entries.iter().find(|e| e.cmd == cmd && e.status == status) else {
            panic!("no {status} {cmd} entry");
        };
        entry
    }

    #[test]
    fn entries_are_sorted_newest_first_across_both_logs() {
        let ts: Vec<String> = parse_activity(SAMPLE).into_iter().map(|e| e.ts).collect();
        let mut sorted = ts.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(ts, sorted);
    }

    #[test]
    fn successful_checks_are_dropped() {
        assert!(!parse_activity(SAMPLE).iter().any(|e| e.cmd == "check"));
    }

    #[test]
    fn installed_skills_are_subjects() {
        let entries = parse_activity(SAMPLE);
        assert_eq!(
            find(&entries, "install", "ok").subjects,
            ["agents-context-router"]
        );
    }

    #[test]
    fn failed_install_falls_back_to_its_source() {
        let entries = parse_activity(SAMPLE);
        assert_eq!(
            find(&entries, "install", "error").subjects,
            ["runkids/skillshare"]
        );
    }

    #[test]
    fn flag_names_and_target_paths_are_not_subjects() {
        let entries = parse_activity(SAMPLE);
        assert_eq!(
            [
                find(&entries, "update", "partial").subjects.clone(),
                find(&entries, "target", "ok").subjects.clone(),
            ],
            [Vec::<String>::new(), vec!["codex".to_string()]]
        );
    }

    #[test]
    fn update_detail_counts_failures() {
        let entries = parse_activity(SAMPLE);
        assert_eq!(
            find(&entries, "update", "partial").detail.as_deref(),
            Some("16 skills, 2 failed")
        );
    }

    #[test]
    fn sync_detail_counts_targets() {
        let entries = parse_activity(SAMPLE);
        assert_eq!(
            find(&entries, "sync", "ok").detail.as_deref(),
            Some("7 targets")
        );
    }

    #[test]
    fn error_keeps_its_message() {
        let entries = parse_activity(SAMPLE);
        assert!(find(&entries, "init", "error")
            .message
            .as_deref()
            .is_some_and(|m| m.starts_with("skillshare is already initialized")));
    }

    #[test]
    fn malformed_lines_and_empty_output_are_skipped() {
        assert!(parse_activity(
            "\nnot json\n{\"ts\":\"bad\",\"cmd\":\"sync\",\"status\":\"ok\"}\n"
        )
        .is_empty());
    }
}
