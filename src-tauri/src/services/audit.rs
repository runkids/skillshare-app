//! Security audit after app-initiated changes that bring in third-party content
//! (Update All, Pull, auto-sync). It only reports: the change is never blocked or undone.

use crate::services::{cli_manager, project_store};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

/// Emitted with the current findings (`Vec<AuditFinding>`) after each audit.
pub const AUDIT_EVENT: &str = "audit-report";

/// The audit reads local files only, but never let a stuck CLI hang the caller.
const AUDIT_TIMEOUT: Duration = Duration::from_secs(60);

/// Severities shown in the app, most severe first; INFO is left to the CLI's audit page.
const SEVERITIES: [&str; 4] = ["CRITICAL", "HIGH", "MEDIUM", "LOW"];

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditFinding {
    pub skill: String,
    /// `skill` or `agent`.
    pub kind: String,
    pub severity: String,
    pub message: String,
    pub file: String,
    /// 0 when the finding is about the file or skill as a whole.
    pub line: u32,
    #[serde(skip)]
    fingerprint: String,
}

/// The latest findings for the active project, most severe first.
#[derive(Default)]
pub struct AuditState(pub tokio::sync::Mutex<Vec<AuditFinding>>);

/// Serializes audits, so overlapping triggers (Update All, then the auto-sync it causes)
/// publish and notify in order.
static AUDIT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(serde::Deserialize)]
struct RawFinding {
    severity: String,
    message: String,
    #[serde(default)]
    file: String,
    #[serde(default)]
    line: u32,
    fingerprint: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawResult {
    skill_name: String,
    #[serde(default)]
    kind: String,
    findings: Option<Vec<RawFinding>>,
}

#[derive(serde::Deserialize)]
struct RawReport {
    results: Option<Vec<RawResult>>,
}

fn rank(severity: &str) -> usize {
    SEVERITIES
        .iter()
        .position(|s| *s == severity)
        .unwrap_or(SEVERITIES.len())
}

/// Findings from `skillshare audit --format json`, without INFO, most severe first.
fn parse_audit(stdout: &str) -> Option<Vec<AuditFinding>> {
    let report: RawReport = serde_json::from_str(stdout)
        .map_err(|e| log::warn!("Unexpected `skillshare audit --format json` output: {e}"))
        .ok()?;
    let mut findings: Vec<AuditFinding> = report
        .results
        .unwrap_or_default()
        .into_iter()
        .flat_map(|r| {
            let (skill, kind) = (r.skill_name, r.kind);
            r.findings
                .unwrap_or_default()
                .into_iter()
                .map(move |f| AuditFinding {
                    skill: skill.clone(),
                    kind: kind.clone(),
                    severity: f.severity,
                    message: f.message,
                    file: f.file,
                    line: f.line,
                    fingerprint: f.fingerprint,
                })
        })
        .filter(|f| rank(&f.severity) < SEVERITIES.len())
        .collect();
    findings.sort_by(|a, b| {
        (rank(&a.severity), &a.skill, &a.file, a.line).cmp(&(
            rank(&b.severity),
            &b.skill,
            &b.file,
            b.line,
        ))
    });
    Some(findings)
}

fn is_alert(finding: &AuditFinding) -> bool {
    rank(&finding.severity) < 2
}

/// The HIGH and CRITICAL findings the last notification didn't cover. Remembers the
/// current ones either way, so a finding that is fixed and comes back is announced anew.
fn unannounced<'a>(
    findings: &'a [AuditFinding],
    notified: &mut Vec<String>,
) -> Vec<&'a AuditFinding> {
    let alerts: Vec<(String, &AuditFinding)> = findings
        .iter()
        .filter(|f| is_alert(f))
        .map(|f| (format!("{}:{}:{}", f.kind, f.skill, f.fingerprint), f))
        .collect();
    let new = alerts
        .iter()
        .filter(|(key, _)| !notified.contains(key))
        .map(|(_, f)| *f)
        .collect();
    *notified = alerts.into_iter().map(|(key, _)| key).collect();
    new
}

fn notification_body(new: &[&AuditFinding]) -> String {
    let body = match new {
        [one] => format!("{} in {}: {}", one.severity, one.skill, one.message),
        _ => {
            let mut skills: Vec<&str> = Vec::new();
            for f in new {
                if !skills.contains(&f.skill.as_str()) {
                    skills.push(&f.skill);
                }
            }
            format!(
                "{} high or critical findings in {}. Open the app to review them.",
                new.len(),
                skills.join(", ")
            )
        }
    };
    body.chars().take(200).collect()
}

/// Run one audit command; it exits non-zero when a finding reaches the CLI's block
/// threshold, but the JSON report is on stdout either way.
async fn audit_json(cli: &str, args: &[&str], dir: Option<&str>) -> Option<Vec<AuditFinding>> {
    let mut cmd = tokio::process::Command::new(cli);
    cmd.args(args)
        .envs(crate::utils::env::build_env_for_child())
        .kill_on_drop(true);
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    let output = match tokio::time::timeout(AUDIT_TIMEOUT, cmd.output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            log::warn!("Audit: failed to run the CLI: {e}");
            return None;
        }
        Err(_) => {
            log::warn!("Audit: timed out after {}s", AUDIT_TIMEOUT.as_secs());
            return None;
        }
    };
    let found = parse_audit(&String::from_utf8_lossy(&output.stdout));
    if found.is_none() {
        log::warn!(
            "Audit failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    found
}

/// Audit every skill and agent in the active project, publish the findings, and notify
/// once per new HIGH or CRITICAL finding. A failed audit keeps the last findings.
pub async fn run(app: &AppHandle) {
    let _running = AUDIT_LOCK.lock().await;
    let Some(cli) = cli_manager::detect_cli().await else {
        return;
    };
    let (dir, is_project) = project_store::active_project_mode(&project_store::load());
    let mode = if is_project { "--project" } else { "--global" };
    // The whole source, not just the changed skills: Pull and auto-sync don't know which
    // skills changed, and a name that an update removed would fail the whole audit.
    let Some(mut findings) = audit_json(
        &cli,
        &["audit", "--format", "json", "--yes", mode],
        dir.as_deref(),
    )
    .await
    else {
        return;
    };
    let Some(agents) = audit_json(
        &cli,
        &["audit", "agents", "--format", "json", "--yes", mode],
        dir.as_deref(),
    )
    .await
    else {
        return;
    };
    findings.extend(agents);
    findings.sort_by_key(|f| rank(&f.severity));

    *app.state::<AuditState>().0.lock().await = findings.clone();
    crate::tray::refresh_audit(app, &findings);
    if let Err(e) = app.emit(AUDIT_EVENT, &findings) {
        log::warn!("Failed to emit audit findings: {e}");
    }

    let mut meta = cli_manager::load_meta();
    if !meta.notify_sync.unwrap_or(true) {
        return;
    }
    let before = meta.notified_audit_findings.clone();
    let new = unannounced(&findings, &mut meta.notified_audit_findings);
    if !new.is_empty() {
        let _ = app
            .notification()
            .builder()
            .title("Skillshare Security Findings")
            .body(notification_body(&new))
            .show();
    }
    if meta.notified_audit_findings != before {
        if let Err(e) = cli_manager::save_meta(&meta) {
            log::warn!("Could not save notified audit findings: {e}");
        }
    }
}

/// The active project changed: the findings belonged to the previous one. Waits for a
/// running audit, so its result for the previous project is cleared too.
pub fn project_changed(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _running = AUDIT_LOCK.lock().await;
        app.state::<AuditState>().0.lock().await.clear();
        crate::tray::refresh_audit(&app, &[]);
        let _ = app.emit(AUDIT_EVENT, Vec::<AuditFinding>::new());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT: &str = r#"{
      "results": [
        {"skillName": "clean", "kind": "skill", "findings": null},
        {"skillName": "pdf", "kind": "skill", "findings": [
          {"severity": "INFO", "message": "interpreter", "file": ".", "line": 0, "fingerprint": "i1"},
          {"severity": "LOW", "message": "shell", "file": "SKILL.md", "line": 3, "fingerprint": "l1"},
          {"severity": "CRITICAL", "message": "Prompt injection", "file": "SKILL.md", "line": 28, "fingerprint": "c1"}
        ]},
        {"skillName": "frontend/doctor", "kind": "skill", "findings": [
          {"severity": "HIGH", "message": "eval", "file": "AUDIT.md", "line": 98, "fingerprint": "h1"},
          {"severity": "MEDIUM", "message": "image", "file": "README.md", "line": 1, "fingerprint": "m1"}
        ]}
      ],
      "summary": {"scanned": 3}
    }"#;

    /// Empty if parsing fails, which fails every assertion below.
    fn report() -> Vec<AuditFinding> {
        parse_audit(REPORT).unwrap_or_default()
    }

    fn severities(findings: &[AuditFinding]) -> Vec<&str> {
        findings.iter().map(|f| f.severity.as_str()).collect()
    }

    #[test]
    fn parses_findings_without_info_most_severe_first() {
        let findings = report();
        assert_eq!(severities(&findings), ["CRITICAL", "HIGH", "MEDIUM", "LOW"]);
    }

    #[test]
    fn findings_keep_their_skill_and_location() {
        let findings = report();
        assert_eq!(
            (
                findings[1].skill.as_str(),
                findings[1].file.as_str(),
                findings[1].line
            ),
            ("frontend/doctor", "AUDIT.md", 98)
        );
    }

    #[test]
    fn empty_results_mean_no_findings() {
        assert_eq!(
            parse_audit(r#"{"results": [], "summary": {}}"#),
            Some(vec![])
        );
    }

    #[test]
    fn cli_errors_are_not_a_clean_report() {
        assert_eq!(parse_audit("✗ project config not found"), None);
    }

    #[test]
    fn only_high_and_critical_findings_are_announced() {
        let findings = report();
        let new = unannounced(&findings, &mut Vec::new());
        assert_eq!(
            severities(&new.into_iter().cloned().collect::<Vec<_>>()),
            ["CRITICAL", "HIGH"]
        );
    }

    #[test]
    fn each_finding_is_announced_once() {
        let findings = report();
        let mut notified = Vec::new();
        unannounced(&findings, &mut notified);
        assert!(unannounced(&findings, &mut notified).is_empty());
    }

    #[test]
    fn a_fixed_finding_that_returns_is_announced_again() {
        let findings = report();
        let mut notified = Vec::new();
        unannounced(&findings, &mut notified);
        unannounced(&[], &mut notified);
        assert_eq!(unannounced(&findings, &mut notified).len(), 2);
    }

    #[test]
    fn the_same_finding_in_another_skill_is_new() {
        let findings = report();
        let mut notified = Vec::new();
        unannounced(&findings[..1], &mut notified);
        let mut copy = findings[0].clone();
        copy.skill = "other".into();
        assert_eq!(unannounced(&[copy], &mut notified).len(), 1);
    }

    #[test]
    fn one_finding_is_named_in_the_notification() {
        let findings = report();
        assert_eq!(
            notification_body(&[&findings[0]]),
            "CRITICAL in pdf: Prompt injection"
        );
    }

    #[test]
    fn several_findings_are_summarized_by_skill() {
        let findings = report();
        assert_eq!(
            notification_body(&[&findings[0], &findings[1]]),
            "2 high or critical findings in pdf, frontend/doctor. Open the app to review them."
        );
    }
}
