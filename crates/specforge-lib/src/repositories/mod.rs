// Repository Layer
// Provides data access abstractions for SQLite database

pub mod ai_conversation_repo;
pub mod ai_repo;
pub mod deploy_repo;
pub mod lockfile_validation_repo;
pub mod mcp_action_repo;
pub mod mcp_repo;
pub mod notification_repo;
pub mod project_repo;
pub mod security_repo;
pub mod settings_repo;
pub mod snapshot_repo;
pub mod template_repo;
pub mod workflow_repo;

// Re-export commonly used repositories
pub use ai_conversation_repo::AIConversationRepository;
pub use ai_repo::AIRepository;
pub use deploy_repo::DeployRepository;
pub use lockfile_validation_repo::LockfileValidationRepository;
pub use mcp_action_repo::MCPActionRepository;
pub use mcp_repo::{MCPRepository, McpLogEntry};
pub use notification_repo::{NotificationListResponse, NotificationRecord, NotificationRepository};
pub use project_repo::ProjectRepository;
pub use security_repo::SecurityRepository;
pub use settings_repo::{
    RecentTemplateEntry, SettingsRepository, TemplatePreferences, TemplateViewMode,
};
pub use snapshot_repo::SnapshotRepository;
pub use template_repo::TemplateRepository;
pub use workflow_repo::WorkflowRepository;

// Note: ExecutionRepository is in src-tauri/src/repositories/ because it depends on commands
