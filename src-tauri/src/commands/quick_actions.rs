use crate::services::{cli_manager, project_store, quick_actions};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "PascalCase", serialize = "camelCase"))]
pub struct SearchResult {
    name: String,
    #[serde(default)]
    description: String,
    source: String,
    #[serde(default)]
    skill: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    project_id: String,
    project_name: String,
    source_dir: String,
}

struct CliContext {
    cli_path: String,
    project_id: String,
    project_name: String,
    working_dir: String,
    mode: &'static str,
}

async fn cli_context(expected_id: Option<&str>) -> Result<CliContext, String> {
    let store = project_store::load();
    let project = store.active_project().ok_or("Set up a project first")?;
    if expected_id.is_some_and(|id| id != project.id) {
        return Err("The active project changed. Reopen Quick Actions before continuing.".into());
    }
    crate::utils::env::load_login_shell_path().await;
    let cli_path = tokio::time::timeout(Duration::from_secs(10), cli_manager::detect_cli())
        .await
        .map_err(|_| "CLI detection timed out")?
        .ok_or("Skillshare CLI not found")?;
    Ok(CliContext {
        cli_path,
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        working_dir: project.path.clone(),
        mode: if project.project_type == crate::models::project::ProjectType::Project {
            "--project"
        } else {
            "--global"
        },
    })
}

async fn run(context: &CliContext, args: Vec<String>, seconds: u64) -> Result<String, String> {
    tokio::time::timeout(
        Duration::from_secs(seconds),
        cli_manager::exec(&context.cli_path, &args, Some(&context.working_dir)),
    )
    .await
    .map_err(|_| format!("Quick action timed out after {seconds}s"))?
}

async fn source_dir(context: &CliContext) -> Result<String, String> {
    let output = run(
        context,
        vec!["status".into(), "--json".into(), context.mode.into()],
        15,
    )
    .await?;
    let value: serde_json::Value = serde_json::from_str(&output).map_err(|e| e.to_string())?;
    value["source"]["path"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "CLI status is missing source.path".into())
}

fn parse_search(output: &str) -> Result<Vec<SearchResult>, String> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Response {
        Results(Vec<SearchResult>),
        Failure { error: String },
    }
    match serde_json::from_str::<Option<Response>>(output)
        .map_err(|e| format!("Invalid search results: {e}"))?
    {
        Some(Response::Results(results)) => Ok(results),
        Some(Response::Failure { error }) => Err(error),
        None => Ok(Vec::new()),
    }
}

fn install_args(source: &str, skill: &str, mode: &str) -> Result<Vec<String>, String> {
    if source.trim().is_empty() || source.starts_with('-') {
        return Err("Invalid skill source".into());
    }
    let mut args = vec![
        "install".into(),
        source.into(),
        "--kind".into(),
        "skill".into(),
        mode.into(),
    ];
    if skill.is_empty() {
        args.push("--yes".into());
    } else {
        args.extend(["--skill".into(), skill.into()]);
    }
    Ok(args)
}

#[tauri::command]
pub fn get_quick_actions_settings(
    app: tauri::AppHandle,
) -> Result<quick_actions::Settings, String> {
    quick_actions::settings(&app)
}

#[tauri::command]
pub fn set_quick_actions_settings(
    app: tauri::AppHandle,
    enabled: bool,
    shortcut: String,
) -> Result<(), String> {
    quick_actions::configure(&app, enabled, shortcut.trim())
}

/// Open the palette from the title bar, the same way the shortcut and the tray do.
#[tauri::command]
pub fn open_quick_actions(app: tauri::AppHandle) {
    quick_actions::request_open(&app);
}

#[tauri::command]
pub fn close_quick_actions(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window(quick_actions::WINDOW_LABEL) {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_quick_actions_context() -> Result<Context, String> {
    let context = cli_context(None).await?;
    Ok(Context {
        source_dir: source_dir(&context).await?,
        project_id: context.project_id,
        project_name: context.project_name,
    })
}

#[tauri::command]
pub async fn quick_search(query: String, project_id: String) -> Result<Vec<SearchResult>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    if query.starts_with('-') {
        return Err("Search must not start with a hyphen".into());
    }
    let context = cli_context(Some(&project_id)).await?;
    let output = run(
        &context,
        vec![
            "search".into(),
            "--json".into(),
            query.into(),
            context.mode.into(),
        ],
        20,
    )
    .await?;
    parse_search(&output)
}

#[tauri::command]
pub async fn quick_install(
    app: tauri::AppHandle,
    source: String,
    skill: String,
    project_id: String,
) -> Result<String, String> {
    let context = cli_context(Some(&project_id)).await?;
    let output = run(&context, install_args(&source, &skill, context.mode)?, 120).await?;
    // Third-party content just arrived: audit it without holding up the palette.
    tauri::async_runtime::spawn(async move { crate::services::audit::run(&app).await });
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cli_search_and_preserves_source_and_selector() {
        let results = parse_search(r#"[{"Name":"pdf","Description":"Read PDFs","Source":"org/skills","Skill":"pdf","Stars":42}]"#).unwrap_or_default();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "org/skills");
        assert_eq!(results[0].skill, "pdf");
        assert!(parse_search("not json").is_err());
        assert!(parse_search(r#"[{"Name":"pdf"}]"#).is_err());
        assert!(parse_search("[]").unwrap_or_default().is_empty());
        assert!(parse_search("null").unwrap_or_default().is_empty());
        assert_eq!(
            parse_search(r#"{"error":"GitHub rate limit exceeded"}"#).err(),
            Some("GitHub rate limit exceeded".into())
        );
    }

    #[test]
    fn noninteractive_install_does_not_force_or_skip_audit() {
        let args = install_args("org/repo/skills/pdf", "", "--project").unwrap_or_default();
        assert_eq!(
            args,
            [
                "install",
                "org/repo/skills/pdf",
                "--kind",
                "skill",
                "--project",
                "--yes"
            ]
        );
        let selected = install_args("org/repo", "pdf", "--global").unwrap_or_default();
        assert_eq!(&selected[selected.len() - 2..], ["--skill", "pdf"]);
        assert!(!selected.iter().any(|arg| arg == "--yes"));
        assert!(install_args("--force", "", "--global").is_err());
        assert!(install_args("", "", "--global").is_err());
    }
}
