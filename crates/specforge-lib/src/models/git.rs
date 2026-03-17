// Git data models
// Models for Git integration feature (009-git-integration)

use serde::{Deserialize, Serialize};

/// Git file status types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GitFileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    Conflict,
}

impl GitFileStatus {
    /// Parse from porcelain v2 status code
    pub fn from_porcelain(code: char) -> Option<Self> {
        match code {
            'M' => Some(GitFileStatus::Modified),
            'A' => Some(GitFileStatus::Added),
            'D' => Some(GitFileStatus::Deleted),
            'R' => Some(GitFileStatus::Renamed),
            'C' => Some(GitFileStatus::Copied),
            '?' => Some(GitFileStatus::Untracked),
            '!' => Some(GitFileStatus::Ignored),
            'U' => Some(GitFileStatus::Conflict),
            _ => None,
        }
    }
}

/// A single file's Git status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFile {
    /// File path relative to repository root
    pub path: String,
    /// File status type
    pub status: GitFileStatus,
    /// Whether the file is staged
    pub staged: bool,
    /// Original path for renamed files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
}

/// Repository Git status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    /// Current branch name
    pub branch: String,
    /// Whether the working directory is clean
    pub is_clean: bool,
    /// Tracking upstream branch name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    /// Number of commits ahead of upstream
    pub ahead: i32,
    /// Number of commits behind upstream
    pub behind: i32,
    /// Count of staged files
    pub staged_count: i32,
    /// Count of modified but unstaged files
    pub modified_count: i32,
    /// Count of untracked files
    pub untracked_count: i32,
    /// Count of conflict files
    pub conflict_count: i32,
    /// List of all changed files
    pub files: Vec<GitFile>,
}

impl Default for GitStatus {
    fn default() -> Self {
        Self {
            branch: String::new(),
            is_clean: true,
            upstream: None,
            ahead: 0,
            behind: 0,
            staged_count: 0,
            modified_count: 0,
            untracked_count: 0,
            conflict_count: 0,
            files: Vec::new(),
        }
    }
}

/// Branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    /// Branch name
    pub name: String,
    /// Whether this is the current branch
    pub is_current: bool,
    /// Whether this is a remote branch
    pub is_remote: bool,
    /// Tracking upstream branch name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    /// Last commit hash (short, 7 chars)
    pub last_commit_hash: String,
    /// Last commit message (truncated)
    pub last_commit_message: String,
}

/// Commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    /// Full commit hash
    pub hash: String,
    /// Short hash (7 chars)
    pub short_hash: String,
    /// Commit message (first line)
    pub message: String,
    /// Author name
    pub author: String,
    /// Author email
    pub author_email: String,
    /// Commit date (ISO 8601)
    pub date: String,
}

/// Commit file change information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitFile {
    /// File path
    pub path: String,
    /// Change type
    pub status: String, // "added" | "modified" | "deleted" | "renamed"
    /// Lines added
    pub additions: i32,
    /// Lines deleted
    pub deletions: i32,
}

/// Commit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitStats {
    /// Number of files changed
    pub files_changed: i32,
    /// Total lines added
    pub additions: i32,
    /// Total lines deleted
    pub deletions: i32,
}

/// Commit detail with changed files
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetail {
    /// Basic commit info
    pub commit: Commit,
    /// Changed files
    pub files: Vec<CommitFile>,
    /// Change statistics
    pub stats: CommitStats,
}

/// Stash entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stash {
    /// Stash index (0 = most recent)
    pub index: i32,
    /// Stash description message
    pub message: String,
    /// Branch where stash was created
    pub branch: String,
    /// Creation timestamp (ISO 8601)
    pub date: String,
}

// ============================================================================
// Diff Models (Feature 010-git-diff-viewer)
// ============================================================================

/// Diff line type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiffLineType {
    Context,
    Addition,
    Deletion,
}

/// A single line in a diff hunk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    /// Line index within hunk (0-based)
    pub index: i32,
    /// Line type
    pub line_type: DiffLineType,
    /// Line content (without prefix)
    pub content: String,
    /// Old file line number (for context/deletion)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_line_number: Option<i32>,
    /// New file line number (for context/addition)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_line_number: Option<i32>,
}

/// A contiguous section of changes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
    /// Hunk index (0-based)
    pub index: i32,
    /// Old file start line
    pub old_start: i32,
    /// Old file line count
    pub old_count: i32,
    /// New file start line
    pub new_start: i32,
    /// New file line count
    pub new_count: i32,
    /// Hunk header (e.g., function name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    /// Lines in this hunk
    pub lines: Vec<DiffLine>,
}

/// File diff status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileDiffStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// Complete diff for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    /// File path relative to repository root
    pub path: String,
    /// Old path for renamed files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    /// File status
    pub status: FileDiffStatus,
    /// Whether file is binary
    pub is_binary: bool,
    /// Detected language for syntax highlighting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Diff hunks
    pub hunks: Vec<DiffHunk>,
    /// Total lines added
    pub additions: i32,
    /// Total lines deleted
    pub deletions: i32,
}
