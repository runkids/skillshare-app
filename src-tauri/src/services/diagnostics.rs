use crate::models::app_state::CliMeta;
use crate::services::server_manager::tail;

/// Log lines kept from each log file in the report.
const LOG_TAIL_LINES: usize = 200;

/// Everything a diagnostics report covers, gathered by the caller.
pub struct DiagnosticsInput {
    pub app_version: String,
    pub os: String,
    pub arch: String,
    pub meta: CliMeta,
    pub server_running: bool,
    pub server_port: u16,
    pub login_shell_path: Option<String>,
    pub app_log: String,
    pub server_log: String,
}

/// Plain-text report for bug reports. The user's home path becomes `~`, so the
/// report does not reveal their account name.
pub fn build_report(input: &DiagnosticsInput, home: Option<&str>) -> String {
    let meta = &input.meta;
    let unknown = || "unknown".to_string();
    let settings = serde_json::to_string_pretty(meta).unwrap_or_else(|e| e.to_string());
    let report = format!(
        "skillshare App diagnostics\n\
         \n\
         App version: {}\n\
         OS: {} ({})\n\
         CLI path: {}\n\
         CLI version: {}\n\
         CLI source: {}\n\
         Server: {} on port {}\n\
         Login shell PATH: {}\n\
         \n\
         == Settings ==\n{settings}\n\
         \n\
         == app.log (last {LOG_TAIL_LINES} lines) ==\n{}\n\
         \n\
         == server.log (last {LOG_TAIL_LINES} lines) ==\n{}\n",
        input.app_version,
        input.os,
        input.arch,
        meta.path.clone().unwrap_or_else(unknown),
        meta.version.clone().unwrap_or_else(unknown),
        meta.source.clone().unwrap_or_else(unknown),
        if input.server_running {
            "running"
        } else {
            "stopped"
        },
        input.server_port,
        input.login_shell_path.clone().unwrap_or_else(unknown),
        tail(
            &crate::services::cli_manager::strip_ansi(&input.app_log),
            LOG_TAIL_LINES
        ),
        tail(
            &crate::services::cli_manager::strip_ansi(&input.server_log),
            LOG_TAIL_LINES
        ),
    );
    match home {
        Some(home) if !home.is_empty() => redact_home(&report, home),
        _ => report,
    }
}

/// Replace the user's home directory with `~`.
pub fn redact_home(text: &str, home: &str) -> String {
    text.replace(home.trim_end_matches('/'), "~")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> DiagnosticsInput {
        DiagnosticsInput {
            app_version: "0.0.13".into(),
            os: "macos".into(),
            arch: "aarch64".into(),
            meta: CliMeta {
                path: Some("/Users/alice/.local/bin/skillshare".into()),
                version: Some("0.19.0".into()),
                ..CliMeta::default()
            },
            server_running: true,
            server_port: 19420,
            login_shell_path: Some("/Users/alice/bin:/usr/bin".into()),
            app_log: (1..=250).map(|i| format!("app line {i}\n")).collect(),
            server_log: String::new(),
        }
    }

    #[test]
    fn report_replaces_home_with_tilde() {
        let report = build_report(&input(), Some("/Users/alice"));
        assert!(!report.contains("/Users/alice"));
    }

    #[test]
    fn report_shows_redacted_cli_path() {
        let report = build_report(&input(), Some("/Users/alice"));
        assert!(report.contains("CLI path: ~/.local/bin/skillshare"));
    }

    #[test]
    fn report_keeps_only_the_last_log_lines() {
        let report = build_report(&input(), None);
        let lines = (
            !report.contains("app line 50\n"),
            report.contains("app line 51\n"),
        );
        assert_eq!(lines, (true, true));
    }

    #[test]
    fn redact_home_ignores_trailing_slash() {
        assert_eq!(redact_home("/home/bob/x", "/home/bob/"), "~/x");
    }
}
