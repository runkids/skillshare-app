// Deploy data models
// One-Click Deploy feature (015-one-click-deploy)
// Extended with Multi Deploy Accounts (016-multi-deploy-accounts)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported deployment platforms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlatformType {
    GithubPages,
    Netlify,
    CloudflarePages,
}

impl std::fmt::Display for PlatformType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformType::GithubPages => write!(f, "github_pages"),
            PlatformType::Netlify => write!(f, "netlify"),
            PlatformType::CloudflarePages => write!(f, "cloudflare_pages"),
        }
    }
}

/// Deployment environment (production or preview)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentEnvironment {
    #[default]
    Production,
    Preview,
}

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    Queued,
    Building,
    Deploying,
    Ready,
    Failed,
    Cancelled,
}

/// Environment variable for deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVariable {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub is_secret: bool,
}

/// Connected platform account (OAuth) - Legacy structure for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedPlatform {
    pub platform: PlatformType,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub access_token: String,
    pub user_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub connected_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl ConnectedPlatform {
    /// Create a sanitized version without access_token for frontend
    pub fn sanitized(&self) -> Self {
        Self {
            platform: self.platform.clone(),
            access_token: String::new(),
            user_id: self.user_id.clone(),
            username: self.username.clone(),
            avatar_url: self.avatar_url.clone(),
            connected_at: self.connected_at,
            expires_at: self.expires_at,
        }
    }
}

/// Deploy Account - Extended from ConnectedPlatform with multi-account support (016-multi-deploy-accounts)
///
/// T001: Added id, display_name, platform_user_id fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployAccount {
    /// Unique identifier for this account (UUID v4)
    pub id: String,
    /// Platform type (vercel/netlify)
    pub platform: PlatformType,
    /// Platform-specific user ID (for duplicate detection)
    pub platform_user_id: String,
    /// Username from the platform
    pub username: String,
    /// User-defined display name (optional, falls back to username)
    pub display_name: Option<String>,
    /// Avatar URL from the platform
    pub avatar_url: Option<String>,
    /// OAuth access token (never exposed to frontend)
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub access_token: String,
    /// When this account was connected
    pub connected_at: DateTime<Utc>,
    /// Token expiration time (if applicable)
    pub expires_at: Option<DateTime<Utc>>,
}

impl DeployAccount {
    /// Create a new DeployAccount from OAuth result
    pub fn new(
        platform: PlatformType,
        platform_user_id: String,
        username: String,
        avatar_url: Option<String>,
        access_token: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            platform,
            platform_user_id,
            username,
            display_name: None,
            avatar_url,
            access_token,
            connected_at: Utc::now(),
            expires_at,
        }
    }

    /// Create a sanitized version without access_token for frontend
    pub fn sanitized(&self) -> Self {
        Self {
            id: self.id.clone(),
            platform: self.platform.clone(),
            platform_user_id: self.platform_user_id.clone(),
            username: self.username.clone(),
            display_name: self.display_name.clone(),
            avatar_url: self.avatar_url.clone(),
            access_token: String::new(),
            connected_at: self.connected_at,
            expires_at: self.expires_at,
        }
    }

    /// Get display name with fallback to username
    pub fn get_display_name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.username)
    }

    /// Migrate from legacy ConnectedPlatform
    pub fn from_connected_platform(platform: ConnectedPlatform) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            platform: platform.platform,
            platform_user_id: platform.user_id.clone(),
            username: platform.username,
            display_name: None,
            avatar_url: platform.avatar_url,
            access_token: platform.access_token,
            connected_at: platform.connected_at,
            expires_at: platform.expires_at,
        }
    }

    /// Convert to legacy ConnectedPlatform for backward compatibility
    pub fn to_connected_platform(&self) -> ConnectedPlatform {
        ConnectedPlatform {
            platform: self.platform.clone(),
            access_token: self.access_token.clone(),
            user_id: self.platform_user_id.clone(),
            username: self.username.clone(),
            avatar_url: self.avatar_url.clone(),
            connected_at: self.connected_at,
            expires_at: self.expires_at,
        }
    }
}

/// T002: Deploy preferences for default account settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeployPreferences {
    /// Default GitHub Pages account ID
    pub default_github_pages_account_id: Option<String>,
    /// Default Netlify account ID
    pub default_netlify_account_id: Option<String>,
    /// Default Cloudflare Pages account ID
    pub default_cloudflare_pages_account_id: Option<String>,
}

impl DeployPreferences {
    /// Get default account ID for a platform
    pub fn get_default_account_id(&self, platform: &PlatformType) -> Option<&String> {
        match platform {
            PlatformType::GithubPages => self.default_github_pages_account_id.as_ref(),
            PlatformType::Netlify => self.default_netlify_account_id.as_ref(),
            PlatformType::CloudflarePages => self.default_cloudflare_pages_account_id.as_ref(),
        }
    }

    /// Set default account ID for a platform
    pub fn set_default_account_id(&mut self, platform: &PlatformType, account_id: Option<String>) {
        match platform {
            PlatformType::GithubPages => self.default_github_pages_account_id = account_id,
            PlatformType::Netlify => self.default_netlify_account_id = account_id,
            PlatformType::CloudflarePages => self.default_cloudflare_pages_account_id = account_id,
        }
    }

    /// Clear default if it matches the given account ID
    pub fn clear_if_matches(&mut self, account_id: &str) {
        if self.default_github_pages_account_id.as_deref() == Some(account_id) {
            self.default_github_pages_account_id = None;
        }
        if self.default_netlify_account_id.as_deref() == Some(account_id) {
            self.default_netlify_account_id = None;
        }
        if self.default_cloudflare_pages_account_id.as_deref() == Some(account_id) {
            self.default_cloudflare_pages_account_id = None;
        }
    }
}

/// T003: Result from removing an account
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAccountResult {
    /// Whether the removal was successful
    pub success: bool,
    /// List of project IDs that were using this account
    pub affected_projects: Vec<String>,
}

/// Deployment configuration per project
/// T005: Added account_id field for project-account binding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentConfig {
    pub project_id: String,
    pub platform: PlatformType,
    /// Bound account ID for this project (016-multi-deploy-accounts)
    pub account_id: Option<String>,
    #[serde(default)]
    pub environment: DeploymentEnvironment,
    pub framework_preset: Option<String>,
    #[serde(default)]
    pub env_variables: Vec<EnvVariable>,
    pub root_directory: Option<String>,
    /// Custom install command (used for GitHub Actions workflow generation)
    pub install_command: Option<String>,
    /// Custom build command (e.g., "pnpm build", "yarn build:prod")
    /// If not set, defaults to "npm run build"
    pub build_command: Option<String>,
    /// Custom output directory (overrides framework preset detection)
    pub output_directory: Option<String>,
    /// Netlify site ID (for reusing existing site across deployments)
    #[serde(default)]
    pub netlify_site_id: Option<String>,
    /// Custom Netlify site name (e.g., "my-awesome-app" for my-awesome-app.netlify.app)
    #[serde(default)]
    pub netlify_site_name: Option<String>,
    /// Cloudflare account ID (required for Cloudflare Pages)
    #[serde(default)]
    pub cloudflare_account_id: Option<String>,
    /// Cloudflare project name (e.g., "my-app" for my-app.pages.dev)
    #[serde(default)]
    pub cloudflare_project_name: Option<String>,
}

/// Single deployment record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    pub id: String,
    pub project_id: String,
    pub platform: PlatformType,
    pub status: DeploymentStatus,
    pub url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub commit_hash: Option<String>,
    pub commit_message: Option<String>,
    pub error_message: Option<String>,
    // Netlify-specific fields
    /// Netlify admin dashboard URL
    #[serde(default)]
    pub admin_url: Option<String>,
    /// Build time in seconds
    #[serde(default)]
    pub deploy_time: Option<u64>,
    /// Branch that was deployed
    #[serde(default)]
    pub branch: Option<String>,
    /// Site name (e.g., "my-app" for my-app.netlify.app)
    #[serde(default)]
    pub site_name: Option<String>,
    /// Unique preview URL for this specific deploy
    #[serde(default)]
    pub preview_url: Option<String>,
}

impl Deployment {
    pub fn new(project_id: String, platform: PlatformType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            project_id,
            platform,
            status: DeploymentStatus::Queued,
            url: None,
            created_at: Utc::now(),
            completed_at: None,
            commit_hash: None,
            commit_message: None,
            error_message: None,
            admin_url: None,
            deploy_time: None,
            branch: None,
            site_name: None,
            preview_url: None,
        }
    }
}

/// OAuth flow result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlowResult {
    pub success: bool,
    pub platform: Option<ConnectedPlatform>,
    pub error: Option<String>,
}

/// Deployment status event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentStatusEvent {
    pub deployment_id: String,
    pub status: DeploymentStatus,
    pub url: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubWorkflowResult {
    /// Whether the workflow file was generated successfully
    pub success: bool,
    /// Path to the generated workflow file
    pub workflow_path: String,
    /// Setup instructions for the user
    pub setup_instructions: Vec<String>,
    /// GitHub username, if detected
    pub username: Option<String>,
    /// GitHub repository name, if detected
    pub repo: Option<String>,
}

/// Result from Cloudflare API token validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudflareValidationResult {
    pub valid: bool,
    /// Cloudflare account ID
    pub account_id: Option<String>,
    /// Account name from Cloudflare
    pub account_name: Option<String>,
    pub error: Option<String>,
}

// ============================================================================
// Deploy UI Enhancement Models (018-deploy-ui-enhancement)
// ============================================================================

/// Deployment statistics for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentStats {
    /// Total number of deployments
    pub total_deployments: usize,
    /// Number of successful deployments
    pub successful_deployments: usize,
    /// Number of failed deployments
    pub failed_deployments: usize,
    /// Success rate as percentage (0-100)
    pub success_rate: f64,
    /// Average deploy time in seconds (for successful deployments)
    pub average_deploy_time: Option<f64>,
    /// Fastest deploy time in seconds
    pub fastest_deploy_time: Option<u64>,
    /// Slowest deploy time in seconds
    pub slowest_deploy_time: Option<u64>,
    /// Last successful deployment info
    pub last_successful_deployment: Option<LastSuccessfulDeployment>,
    /// Deployments in the last 7 days
    pub recent_deployments_count: usize,
}

/// Info about the last successful deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastSuccessfulDeployment {
    pub id: String,
    pub url: String,
    pub deployed_at: DateTime<Utc>,
    pub commit_hash: Option<String>,
    pub platform: PlatformType,
}

/// Extended site information from Netlify API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetlifySiteInfo {
    pub site_id: String,
    pub name: String,
    pub url: String,
    pub ssl_url: String,
    pub screenshot_url: Option<String>,
    pub custom_domain: Option<String>,
    pub ssl: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub repo_url: Option<String>,
    pub repo_branch: Option<String>,
    pub build_minutes_used: Option<u64>,
    pub build_minutes_included: Option<u64>,
    pub form_count: Option<usize>,
    pub account_slug: Option<String>,
    pub account_name: Option<String>,
}

/// Extended project information from Cloudflare Pages API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudflareProjectInfo {
    pub name: String,
    pub subdomain: String,
    pub domains: Vec<String>,
    pub production_branch: String,
    pub latest_deployment_url: Option<String>,
    pub latest_deployment_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub deployments_count: Option<usize>,
}

/// GitHub Pages site information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubPagesInfo {
    pub url: String,
    pub status: Option<String>,
    pub branch: String,
    pub path: String,
    pub https_enforced: bool,
    pub custom_domain: Option<String>,
    pub latest_workflow_status: Option<String>,
    pub latest_workflow_conclusion: Option<String>,
    pub latest_workflow_url: Option<String>,
}

/// Union type for platform-specific info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "platform")]
pub enum PlatformSiteInfo {
    #[serde(rename = "netlify")]
    Netlify { info: NetlifySiteInfo },
    #[serde(rename = "cloudflare_pages")]
    CloudflarePages { info: CloudflareProjectInfo },
    #[serde(rename = "github_pages")]
    GithubPages { info: GitHubPagesInfo },
}

/// Extended deployment status event with progress info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentProgressEvent {
    pub deployment_id: String,
    pub status: DeploymentStatus,
    /// Progress percentage (0-100), if available
    pub progress: Option<u8>,
    /// Current step name
    pub current_step: Option<String>,
    /// Total steps
    pub total_steps: Option<u8>,
    /// Current step index (1-based)
    pub current_step_index: Option<u8>,
    /// Elapsed time in seconds
    pub elapsed_seconds: Option<u64>,
    pub url: Option<String>,
    pub error_message: Option<String>,
}
