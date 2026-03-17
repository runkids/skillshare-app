// Worktree session data models
// Represents saved context and resume actions per Git worktree

use serde::{Deserialize, Serialize};

fn default_session_status() -> String {
    "active".to_string()
}

/// A saved unit of context for a specific worktree within a project
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeSession {
    pub id: String,
    pub project_id: String,
    pub worktree_path: String,
    pub branch_snapshot: Option<String>,
    pub title: String,
    pub goal: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub checklist: Vec<SessionChecklistItem>,
    #[serde(default)]
    pub resume_actions: Vec<ResumeAction>,
    #[serde(default = "default_session_status")]
    pub status: String,
    pub broken_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
    pub last_resumed_at: Option<String>,
}

/// A trackable next-step item inside a session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionChecklistItem {
    pub id: String,
    pub text: String,
    pub completed: bool,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// A configured action to run when resuming a session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAction {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: String,
    pub label: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    // openEditor
    pub editor_id: Option<String>,
    // runScript
    pub script_name: Option<String>,
    // runWorkflow
    pub workflow_id: Option<String>,
    pub workflow_name: Option<String>,
    #[serde(default)]
    pub wait_for_completion: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_session_serde_round_trip() {
        let session = WorktreeSession {
            id: "ws-1".to_string(),
            project_id: "p-1".to_string(),
            worktree_path: "/tmp/repo/.worktrees/001-demo".to_string(),
            branch_snapshot: Some("001-demo".to_string()),
            title: "Demo Session".to_string(),
            goal: Some("Ship MVP".to_string()),
            notes: "Some notes".to_string(),
            tags: vec!["mvp".to_string(), "worktree".to_string()],
            checklist: vec![SessionChecklistItem {
                id: "c-1".to_string(),
                text: "Do the thing".to_string(),
                completed: false,
                created_at: "2025-01-01T00:00:00Z".to_string(),
                updated_at: "2025-01-01T00:00:00Z".to_string(),
                completed_at: None,
            }],
            resume_actions: vec![ResumeAction {
                id: "a-1".to_string(),
                action_type: "openEditor".to_string(),
                label: None,
                enabled: true,
                editor_id: Some("vscode".to_string()),
                script_name: None,
                workflow_id: None,
                workflow_name: None,
                wait_for_completion: None,
            }],
            status: "active".to_string(),
            broken_reason: None,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            archived_at: None,
            last_resumed_at: None,
        };

        let json = serde_json::to_string(&session).expect("serialize worktree session");
        let parsed: WorktreeSession =
            serde_json::from_str(&json).expect("deserialize worktree session");

        assert_eq!(parsed, session);
    }
}
