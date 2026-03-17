// Deploy commands
// One-Click Deploy feature (015-one-click-deploy)
// Extended with Multi Deploy Accounts (016-multi-deploy-accounts)
// Secure token storage with AES-256-GCM encryption
// Refactored to use DeployRepository for proper SQLite schema access

use crate::models::deploy::{
    CloudflareValidationResult, ConnectedPlatform, DeployAccount, DeployPreferences, Deployment,
    DeploymentConfig, DeploymentStatus, DeploymentStatusEvent, GitHubWorkflowResult,
    OAuthFlowResult, PlatformType, RemoveAccountResult,
};
use crate::repositories::DeployRepository;
use crate::services::crypto;
use crate::services::crypto::EncryptedData;
use crate::services::deploy as deploy_service;
use crate::services::notification::{send_notification, NotificationType};
use crate::utils::database::Database;
use crate::DatabaseState;
use std::path::Path;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

#[derive(Debug, Serialize)]
pub struct CheckAccountResult {
    in_use: bool,
    affected_projects: Vec<String>,
}

// OAuth client configuration
//
// These values are loaded from environment variables at COMPILE TIME using option_env!
// This ensures they are embedded in the binary for packaged apps.
// Set PACKAGEFLOW_NETLIFY_CLIENT_ID when building for release.
const NETLIFY_CLIENT_ID: Option<&str> = option_env!("PACKAGEFLOW_NETLIFY_CLIENT_ID");

/// OAuth success page HTML - displayed after successful authorization
const OAUTH_SUCCESS_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Authorization Successful - SpecForge</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            color: #e4e4e7;
        }
        .card {
            background: rgba(255, 255, 255, 0.05);
            backdrop-filter: blur(10px);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 16px;
            padding: 48px;
            text-align: center;
            max-width: 420px;
            animation: fadeIn 0.5s ease-out;
        }
        @keyframes fadeIn {
            from { opacity: 0; transform: translateY(20px); }
            to { opacity: 1; transform: translateY(0); }
        }
        .logo {
            width: 72px;
            height: 72px;
            margin: 0 auto 28px;
            border-radius: 16px;
            overflow: hidden;
            animation: scaleIn 0.3s ease-out 0.1s both;
        }
        .logo svg {
            width: 100%;
            height: 100%;
        }
        .success-icon {
            width: 64px;
            height: 64px;
            background: linear-gradient(135deg, #10b981 0%, #059669 100%);
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            margin: 0 auto 24px;
            animation: scaleIn 0.3s ease-out 0.3s both;
        }
        @keyframes scaleIn {
            from { transform: scale(0); }
            to { transform: scale(1); }
        }
        .success-icon svg {
            width: 32px;
            height: 32px;
            stroke: white;
            stroke-width: 3;
            fill: none;
        }
        .success-icon svg path {
            stroke-dasharray: 50;
            stroke-dashoffset: 50;
            animation: checkmark 0.4s ease-out 0.6s forwards;
        }
        @keyframes checkmark {
            to { stroke-dashoffset: 0; }
        }
        h1 {
            font-size: 24px;
            font-weight: 600;
            margin-bottom: 12px;
            color: #f4f4f5;
        }
        p {
            font-size: 15px;
            color: #a1a1aa;
            line-height: 1.6;
        }
        .brand {
            margin-top: 32px;
            padding-top: 24px;
            border-top: 1px solid rgba(255, 255, 255, 0.1);
            font-size: 13px;
            color: #71717a;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 8px;
        }
        .brand-logo {
            width: 20px;
            height: 20px;
            border-radius: 4px;
            overflow: hidden;
        }
        .brand span {
            color: #a1a1aa;
            font-weight: 500;
        }
    </style>
</head>
<body>
    <div class="card">
        <div class="success-icon">
            <svg viewBox="0 0 24 24">
                <path d="M5 13l4 4L19 7"/>
            </svg>
        </div>
        <h1>Authorization Successful</h1>
        <p>Your account has been connected successfully. You can now close this window and return to SpecForge.</p>
        <div class="brand">
            <div class="brand-logo">
                <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
                    <defs>
                        <linearGradient id="bg2" x1="0%" y1="0%" x2="100%" y2="100%">
                            <stop offset="0%" style="stop-color:#4db6ac"/>
                            <stop offset="100%" style="stop-color:#26a69a"/>
                        </linearGradient>
                    </defs>
                    <rect width="100" height="100" rx="20" fill="url(#bg2)"/>
                    <text x="50" y="42" font-family="-apple-system, sans-serif" font-size="32" font-weight="300" fill="rgba(255,255,255,0.7)" text-anchor="middle">&lt;/&gt;</text>
                    <rect x="22" y="55" width="24" height="22" rx="3" fill="white"/>
                    <rect x="54" y="55" width="24" height="22" rx="3" fill="white"/>
                </svg>
            </div>
            <span>SpecForge</span>
        </div>
    </div>
</body>
</html>"##;

enum OAuthClientConfig {
    Netlify { client_id: String },
}

fn get_oauth_client_config(platform: &PlatformType) -> Result<OAuthClientConfig, String> {
    match platform {
        PlatformType::GithubPages => {
            Err("GitHub Pages does not require OAuth. It uses git credentials.".to_string())
        }
        PlatformType::Netlify => {
            let client_id = NETLIFY_CLIENT_ID
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| {
                    "Netlify OAuth is not configured. Set PACKAGEFLOW_NETLIFY_CLIENT_ID.".to_string()
                })?;
            Ok(OAuthClientConfig::Netlify {
                client_id: client_id.to_string(),
            })
        }
        PlatformType::CloudflarePages => {
            Err("Cloudflare Pages uses API Token authentication, not OAuth.".to_string())
        }
    }
}

// API endpoints
const NETLIFY_AUTH_URL: &str = "https://app.netlify.com/authorize";
const NETLIFY_USER_URL: &str = "https://api.netlify.com/api/v1/user";
const NETLIFY_SITES_URL: &str = "https://api.netlify.com/api/v1/sites";

// Cloudflare API endpoints (Phase 3)
const CLOUDFLARE_API_BASE: &str = "https://api.cloudflare.com/client/v4";
const CLOUDFLARE_VERIFY_URL: &str = "https://api.cloudflare.com/client/v4/user/tokens/verify";

// T015: Maximum accounts per platform
const MAX_ACCOUNTS_PER_PLATFORM: usize = 5;

/// Deployment result with optional platform-specific metadata
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
struct DeployResult {
    url: String,
    deploy_id: String,
    // Platform-specific fields (mainly Netlify)
    admin_url: Option<String>,
    deploy_time: Option<u64>,
    site_name: Option<String>,
    preview_url: Option<String>,
    branch: Option<String>,
}

// ============================================================================
// Helper Functions (SQLite-based via DeployRepository)
// ============================================================================

/// Get Database from AppHandle
fn get_db(app: &AppHandle) -> Database {
    let db_state = app.state::<DatabaseState>();
    db_state.0.as_ref().clone()
}

/// Get DeployRepository from AppHandle
fn get_deploy_repo(app: &AppHandle) -> DeployRepository {
    DeployRepository::new(get_db(app))
}

// ============================================================================
// T006, T007: Deploy Account Helper Functions (016-multi-deploy-accounts)
// Now using DeployRepository for proper SQLite schema access
// ============================================================================

/// Get deploy accounts from SQLite
fn get_accounts_from_store(app: &AppHandle) -> Result<Vec<DeployAccount>, String> {
    let repo = get_deploy_repo(app);
    repo.list_accounts()
}

/// Save deploy account to SQLite with encrypted token storage
fn save_account_to_store(app: &AppHandle, account: &DeployAccount) -> Result<(), String> {
    let repo = get_deploy_repo(app);

    // Save account without plaintext token
    let mut account_to_save = account.clone();
    account_to_save.access_token = String::new(); // Don't store plaintext token
    repo.save_account(&account_to_save)?;

    // Encrypt and store token separately if not empty
    if !account.access_token.is_empty() {
        let encrypted = crypto::encrypt(&account.access_token)
            .map_err(|e| format!("Failed to encrypt token: {}", e))?;
        repo.store_token(&account.id, &encrypted.ciphertext, &encrypted.nonce)?;
    }

    Ok(())
}

/// Get decrypted access token for an account
/// Tries encrypted storage first, falls back to legacy plaintext for migration
fn get_decrypted_token(repo: &DeployRepository, account_id: &str) -> Result<Option<String>, String> {
    // Try encrypted token first
    if let Some((ciphertext, nonce)) = repo.get_token(account_id)? {
        let encrypted = crypto::EncryptedData { ciphertext, nonce };
        let decrypted = crypto::decrypt(&encrypted)
            .map_err(|e| format!("Failed to decrypt token: {}", e))?;
        return Ok(Some(decrypted));
    }

    // Fall back to legacy plaintext token for migration
    if let Some(legacy_token) = repo.get_legacy_token(account_id)? {
        // Migrate: encrypt and store in new table, then clear legacy
        log::info!("Migrating deploy token for account {} to encrypted storage", account_id);
        let encrypted = crypto::encrypt(&legacy_token)
            .map_err(|e| format!("Failed to encrypt legacy token: {}", e))?;
        repo.store_token(account_id, &encrypted.ciphertext, &encrypted.nonce)?;
        repo.clear_legacy_token(account_id)?;
        return Ok(Some(legacy_token));
    }

    Ok(None)
}

/// T008: Get deploy preferences from SQLite
fn get_preferences_from_store(app: &AppHandle) -> Result<DeployPreferences, String> {
    let repo = get_deploy_repo(app);
    repo.get_preferences()
}

/// T008: Save deploy preferences to SQLite
fn save_preferences_to_store(app: &AppHandle, prefs: &DeployPreferences) -> Result<(), String> {
    let repo = get_deploy_repo(app);
    repo.save_preferences(prefs)
}

/// Find account by ID using repository
fn find_account_by_id_from_store(app: &AppHandle, account_id: &str) -> Result<Option<DeployAccount>, String> {
    let repo = get_deploy_repo(app);
    repo.get_account(account_id)
}

/// T037: Find projects using a specific account
fn find_projects_using_account(app: &AppHandle, account_id: &str) -> Result<Vec<String>, String> {
    let repo = get_deploy_repo(app);
    repo.find_projects_using_account(account_id)
}

/// Clear account_id from all configs that reference the given account
fn clear_account_from_configs(app: &AppHandle, account_id: &str) -> Result<(), String> {
    let repo = get_deploy_repo(app);
    repo.clear_account_from_configs(account_id)?;
    Ok(())
}

/// T031: Get access token for deployment with priority:
/// 1. Bound account (config.account_id)
/// 2. Default account for the platform
/// 3. Legacy connected platform (backward compatibility)
fn get_deployment_access_token(
    app: &AppHandle,
    config: &DeploymentConfig,
) -> Result<String, String> {
    // GitHub Pages doesn't require OAuth - it uses git credentials
    if config.platform == PlatformType::GithubPages {
        return Ok(String::new());
    }

    let repo = get_deploy_repo(app);
    let prefs = repo.get_preferences()?;

    // 1. Try bound account
    if let Some(account_id) = &config.account_id {
        if repo.get_account(account_id)?.is_some() {
            if let Some(token) = get_decrypted_token(&repo, account_id)? {
                return Ok(token);
            }
        }
    }

    // 2. Try default account for platform
    if let Some(default_id) = prefs.get_default_account_id(&config.platform) {
        if repo.get_account(default_id)?.is_some() {
            if let Some(token) = get_decrypted_token(&repo, default_id)? {
                return Ok(token);
            }
        }
    }

    // 3. Try any account for the platform
    let accounts = repo.list_accounts_by_platform(config.platform.clone())?;
    for account in &accounts {
        if let Some(token) = get_decrypted_token(&repo, &account.id)? {
            return Ok(token);
        }
    }

    // 4. Fall back to legacy connected platform (before multi-account support)
    let connected = check_platform_connected(app, &config.platform)?;
    // Legacy connected platforms don't have account IDs, just return their token directly
    Ok(connected.access_token)
}

/// Get deployment config from SQLite
fn get_config_from_store(app: &AppHandle, project_id: &str) -> Result<Option<DeploymentConfig>, String> {
    println!("[get_config_from_store] project_id={}", project_id);
    let repo = get_deploy_repo(app);
    let result = repo.get_config(project_id);
    match &result {
        Ok(Some(config)) => {
            println!(
                "[get_config_from_store] FOUND config for project_id={}, platform={:?}, account_id={:?}",
                project_id,
                config.platform,
                config.account_id
            );
        }
        Ok(None) => {
            println!("[get_config_from_store] NO CONFIG for project_id={}", project_id);
        }
        Err(e) => {
            println!("[get_config_from_store] ERROR for project_id={}: {}", project_id, e);
        }
    }
    result
}

/// Save deployment config to SQLite
fn save_config_to_store(app: &AppHandle, config: &DeploymentConfig) -> Result<(), String> {
    println!(
        "[save_config_to_store] project_id={}, platform={:?}, account_id={:?}",
        config.project_id,
        config.platform,
        config.account_id
    );

    let db = get_db(app);

    // CRITICAL DEBUG: Print PRAGMA database_list to verify we're using the same DB
    let _ = db.with_connection(|conn| {
        println!("=== [SAVE] PRAGMA database_list (SQLite actual file) ===");
        if let Ok(mut stmt) = conn.prepare("PRAGMA database_list;") {
            let rows: Vec<(i64, String, String)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .map(|iter| iter.filter_map(|r| r.ok()).collect())
                .unwrap_or_default();
            for (seq, name, file) in &rows {
                println!("[SAVE] DB seq={}, name={}, file={}", seq, name, file);
            }
        }
        Ok(())
    });

    let repo = get_deploy_repo(app);
    let result = repo.save_config(config);

    // Force WAL checkpoint to ensure data is persisted
    if result.is_ok() {
        if let Err(e) = db.with_connection(|conn| {
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|e| format!("WAL checkpoint failed: {}", e))
        }) {
            println!("[save_config_to_store] WAL checkpoint warning: {}", e);
        }
        println!("[save_config_to_store] SUCCESS for project_id={}", config.project_id);

        // DEBUG: Verify by querying back with same connection
        let _ = db.with_connection(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM deployment_configs WHERE project_id = ?1",
                    [&config.project_id],
                    |r| r.get(0),
                )
                .unwrap_or(-1);
            println!("[save_config_to_store] VERIFY: config exists in DB = {}", count > 0);

            // List all configs
            let total: i64 = conn
                .query_row("SELECT COUNT(*) FROM deployment_configs", [], |r| r.get(0))
                .unwrap_or(-1);
            println!("[save_config_to_store] TOTAL deployment_configs in DB: {}", total);
            Ok(())
        });
    } else {
        println!("[save_config_to_store] FAILED: {:?}", result);
    }

    result
}

/// Save Netlify site ID to config for reuse across deployments
fn save_netlify_site_id(app: &AppHandle, project_id: &str, site_id: &str) -> Result<(), String> {
    let repo = get_deploy_repo(app);
    if let Some(mut config) = repo.get_config(project_id)? {
        config.netlify_site_id = Some(site_id.to_string());
        repo.save_config(&config)?;
    }
    Ok(())
}

/// Get deployment history from SQLite
fn get_deployments_from_store(app: &AppHandle, project_id: &str) -> Result<Vec<Deployment>, String> {
    let repo = get_deploy_repo(app);
    repo.list_deployments(project_id)
}

/// Save a single deployment to history
fn save_deployment_to_history(app: &AppHandle, deployment: &Deployment) -> Result<(), String> {
    let repo = get_deploy_repo(app);
    repo.save_deployment(deployment)
}

/// Check if a platform is connected - now reads from SQLite
fn check_platform_connected(
    app: &AppHandle,
    platform: &PlatformType,
) -> Result<ConnectedPlatform, String> {
    let repo = get_deploy_repo(app);
    let accounts = repo.list_accounts_by_platform(platform.clone())?;

    // Return the first account found for this platform
    accounts
        .into_iter()
        .next()
        .map(|a| a.to_connected_platform())
        .ok_or_else(|| format!("Platform {} not connected", platform))
}

/// Build Netlify OAuth authorization URL (implicit grant)
fn build_netlify_auth_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    format!(
        "{}?client_id={}&response_type=token&redirect_uri={}&state={}",
        NETLIFY_AUTH_URL,
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(state)
    )
}

// ============================================================================
// OAuth Commands
// ============================================================================

/// Start OAuth flow for a platform
#[tauri::command]
pub async fn start_oauth_flow(
    app: AppHandle,
    platform: PlatformType,
) -> Result<OAuthFlowResult, String> {
    use uuid::Uuid;

    let oauth_config = match get_oauth_client_config(&platform) {
        Ok(config) => config,
        Err(error) => {
            return Ok(OAuthFlowResult {
                success: false,
                platform: None,
                error: Some(error),
            });
        }
    };

    // Generate state for CSRF protection
    let state = Uuid::new_v4().to_string();
    let state_clone = state.clone();

    // Channel to receive the callback URL
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    // Start local OAuth server with callback on fixed port
    let config = tauri_plugin_oauth::OauthConfig {
        ports: Some(vec![8766, 8767, 8768]), // Try these ports in order
        response: Some(OAUTH_SUCCESS_HTML.into()),
    };
    let port = tauri_plugin_oauth::start_with_config(config, move |url| {
        let tx = tx.clone();
        // Send the URL through the channel
        tauri::async_runtime::spawn(async move {
            if let Some(sender) = tx.lock().await.take() {
                let _ = sender.send(url);
            }
        });
    })
    .map_err(|e| format!("Failed to start OAuth server: {}", e))?;

    let redirect_uri = format!("http://localhost:{}/callback", port);

    // Build authorization URL based on platform
    let auth_url = match &oauth_config {
        OAuthClientConfig::Netlify { client_id } => {
            build_netlify_auth_url(client_id, &redirect_uri, &state)
        }
    };

    // Open browser for authorization
    if let Err(e) = opener::open_browser(&auth_url) {
        let _ = tauri_plugin_oauth::cancel(port);
        return Ok(OAuthFlowResult {
            success: false,
            platform: None,
            error: Some(format!("Failed to open browser: {}", e)),
        });
    }

    // Wait for callback with timeout (60 seconds)
    let callback_result = tokio::time::timeout(std::time::Duration::from_secs(60), rx).await;

    // Cancel the OAuth server
    let _ = tauri_plugin_oauth::cancel(port);

    let callback_url = match callback_result {
        Ok(Ok(url)) => url,
        Ok(Err(_)) => {
            return Ok(OAuthFlowResult {
                success: false,
                platform: None,
                error: Some("OAuth callback channel closed".to_string()),
            });
        }
        Err(_) => {
            return Ok(OAuthFlowResult {
                success: false,
                platform: None,
                error: Some("OAuth flow timed out".to_string()),
            });
        }
    };

    // Parse callback URL and exchange for token
    let connected_platform = match oauth_config {
        OAuthClientConfig::Netlify { .. } => {
            extract_netlify_token(&callback_url, &state_clone).await?
        }
    };

    // Convert to DeployAccount and save to SQLite
    let new_account = DeployAccount::from_connected_platform(connected_platform.clone());
    let repo = get_deploy_repo(&app);

    // Check for duplicate account (same platform + platform_user_id)
    if !repo.account_exists_by_platform_user(&platform, &new_account.platform_user_id)? {
        // Save new account to SQLite
        save_account_to_store(&app, &new_account)?;
    }

    Ok(OAuthFlowResult {
        success: true,
        platform: Some(connected_platform.sanitized()),
        error: None,
    })
}

/// Extract Netlify token from redirect URL (implicit grant)
async fn extract_netlify_token(
    callback_url: &str,
    expected_state: &str,
) -> Result<ConnectedPlatform, String> {
    // Netlify uses hash fragment for implicit grant
    // URL format: http://localhost:PORT#access_token=XXX&token_type=Bearer&state=YYY
    let url = url::Url::parse(callback_url).map_err(|e| format!("Invalid callback URL: {}", e))?;

    let fragment = url.fragment().ok_or("Missing URL fragment")?;
    let params: std::collections::HashMap<String, String> = fragment
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect();

    let access_token = params
        .get("access_token")
        .ok_or("Missing access_token in fragment")?
        .to_string();
    let state = params
        .get("state")
        .ok_or("Missing state in fragment")?
        .to_string();

    // Verify state
    if state != expected_state {
        return Err("State mismatch - possible CSRF attack".to_string());
    }

    // Fetch user info
    let client = reqwest::Client::new();
    let user_response = client
        .get(NETLIFY_USER_URL)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch user info: {}", e))?;

    if !user_response.status().is_success() {
        return Err("Failed to fetch Netlify user info".to_string());
    }

    let user_data: serde_json::Value = user_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse user response: {}", e))?;

    let user_id = user_data["id"].as_str().unwrap_or("").to_string();
    let username = user_data["full_name"]
        .as_str()
        .or_else(|| user_data["email"].as_str())
        .unwrap_or("Unknown")
        .to_string();
    let avatar_url = user_data["avatar_url"].as_str().map(|s| s.to_string());

    Ok(ConnectedPlatform {
        platform: PlatformType::Netlify,
        access_token,
        user_id,
        username,
        avatar_url,
        connected_at: chrono::Utc::now(),
        expires_at: None,
    })
}

/// Get all connected platforms (sanitized - no tokens)
/// Now reads from SQLite and converts DeployAccount to ConnectedPlatform for backward compatibility
#[tauri::command]
pub async fn get_connected_platforms(app: AppHandle) -> Result<Vec<ConnectedPlatform>, String> {
    let repo = get_deploy_repo(&app);
    let accounts = repo.list_accounts()?;

    // Convert DeployAccount to ConnectedPlatform for backward compatibility
    Ok(accounts
        .into_iter()
        .map(|a| a.to_connected_platform().sanitized())
        .collect())
}

/// Disconnect a platform - removes all accounts of this platform type from SQLite
#[tauri::command]
pub async fn disconnect_platform(app: AppHandle, platform: PlatformType) -> Result<(), String> {
    let repo = get_deploy_repo(&app);

    // Get all accounts for this platform
    let accounts = repo.list_accounts_by_platform(platform.clone())?;

    // Delete each account
    for account in accounts {
        // Clear from deployment configs first
        repo.clear_account_from_configs(&account.id)?;

        // Clear from preferences if it was a default
        let mut prefs = repo.get_preferences()?;
        prefs.clear_if_matches(&account.id);
        repo.save_preferences(&prefs)?;

        // Delete the account
        repo.delete_account(&account.id)?;
    }

    Ok(())
}

// ============================================================================
// Deployment Commands
// ============================================================================

/// Start a new deployment
/// T031: Updated to use bound account or fall back to default account
#[tauri::command]
pub async fn start_deployment(
    app: AppHandle,
    project_id: String,
    project_path: String,
    config: DeploymentConfig,
) -> Result<Deployment, String> {
    // T031: Get access token from bound account, default account, or legacy connected platform
    let access_token = get_deployment_access_token(&app, &config)?;

    // Create deployment record
    let deployment = Deployment::new(project_id.clone(), config.platform.clone());

    // Save initial deployment to history
    save_deployment_to_history(&app, &deployment)?;

    // Clone values for async task
    let app_clone = app.clone();
    let deployment_id = deployment.id.clone();
    let access_token_clone = access_token.clone();
    let config_clone = config.clone();
    let project_path_clone = project_path.clone();

    // Start deployment in background
    tauri::async_runtime::spawn(async move {
        let result = execute_deployment(
            &app_clone,
            &deployment_id,
            &access_token_clone,
            &config_clone,
            &project_path_clone,
        )
        .await;

        // Extract project name for notifications
        let project_name = Path::new(&project_path_clone)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let platform = config_clone.platform.to_string();

        // Update deployment status based on result using repository
        let repo = get_deploy_repo(&app_clone);
        if let Ok(Some(mut dep)) = repo.get_deployment(&deployment_id) {
            match result {
                Ok(deploy_result) => {
                    dep.status = DeploymentStatus::Ready;
                    dep.url = Some(deploy_result.url.clone());
                    dep.completed_at = Some(chrono::Utc::now());
                    // Store Netlify-specific info
                    dep.admin_url = deploy_result.admin_url;
                    dep.deploy_time = deploy_result.deploy_time;
                    dep.site_name = deploy_result.site_name;
                    dep.preview_url = deploy_result.preview_url;
                    dep.branch = deploy_result.branch;

                    // Emit success event
                    let _ = app_clone.emit(
                        "deployment:status",
                        DeploymentStatusEvent {
                            deployment_id: deployment_id.clone(),
                            status: DeploymentStatus::Ready,
                            url: Some(deploy_result.url),
                            error_message: None,
                        },
                    );

                    // Send desktop notification for deployment success
                    let _ = send_notification(
                        &app_clone,
                        NotificationType::DeploymentSuccess {
                            project_name: project_name.clone(),
                            platform: platform.clone(),
                        },
                    );
                }
                Err(error) => {
                    dep.status = DeploymentStatus::Failed;
                    dep.error_message = Some(error.clone());
                    dep.completed_at = Some(chrono::Utc::now());

                    // Emit failure event
                    let _ = app_clone.emit(
                        "deployment:status",
                        DeploymentStatusEvent {
                            deployment_id: deployment_id.clone(),
                            status: DeploymentStatus::Failed,
                            url: None,
                            error_message: Some(error.clone()),
                        },
                    );

                    // Send desktop notification for deployment failure
                    let _ = send_notification(
                        &app_clone,
                        NotificationType::DeploymentFailed {
                            project_name,
                            platform,
                            error,
                        },
                    );
                }
            }
            let _ = repo.save_deployment(&dep);
        }
    });

    // Return initial deployment record
    Ok(deployment)
}

/// Execute the actual deployment
async fn execute_deployment(
    app: &AppHandle,
    deployment_id: &str,
    access_token: &str,
    config: &DeploymentConfig,
    project_path: &str,
) -> Result<DeployResult, String> {
    // Emit building status
    let _ = app.emit(
        "deployment:status",
        DeploymentStatusEvent {
            deployment_id: deployment_id.to_string(),
            status: DeploymentStatus::Building,
            url: None,
            error_message: None,
        },
    );

    // Determine build output directory: custom > framework preset > default
    let build_dir = config
        .output_directory
        .clone()
        .unwrap_or_else(|| get_build_output_dir(config.framework_preset.as_deref()));
    let full_build_path = std::path::Path::new(project_path).join(&build_dir);

    // Run build command
    run_build_command(project_path, config).await?;

    // Verify build output exists
    if !full_build_path.exists() {
        return Err(format!(
            "Build output directory not found: {}. Please check your build command and output directory settings.",
            full_build_path.display()
        ));
    }

    match config.platform {
        PlatformType::GithubPages => {
            let (url, deploy_id) =
                deploy_to_github_pages(app, deployment_id, project_path, config, &full_build_path)
                    .await?;
            Ok(DeployResult {
                url,
                deploy_id,
                ..Default::default()
            })
        }
        PlatformType::Netlify => {
            deploy_to_netlify(app, deployment_id, access_token, config, &full_build_path).await
        }
        PlatformType::CloudflarePages => {
            // Use the new CloudflareProvider from services::deploy
            let internal_account_id = config
                .account_id
                .as_ref()
                .ok_or("No deploy account is bound to this project")?;

            let account = find_account_by_id_from_store(app, internal_account_id)?
                .ok_or("Bound deploy account not found")?;

            // Create provider with account's platform_user_id (Cloudflare Account ID)
            let provider = deploy_service::create_provider(
                PlatformType::CloudflarePages,
                access_token.to_string(),
                Some(account.platform_user_id.clone()),
            )
            .map_err(|e| e.to_string())?;

            // Execute deployment
            let result = provider
                .deploy(app, deployment_id, config, &full_build_path)
                .await
                .map_err(|e| e.to_string())?;

            Ok(DeployResult {
                url: result.url,
                deploy_id: result.provider_deploy_id.unwrap_or_default(),
                site_name: Some(config.cloudflare_project_name.clone().unwrap_or_default()),
                preview_url: result.alias_url,
                ..Default::default()
            })
        }
    }
}

/// Get the build output directory based on framework preset
fn get_build_output_dir(framework: Option<&str>) -> String {
    match framework {
        Some("nextjs") => ".next".to_string(),
        Some("react") | Some("create-react-app") => "build".to_string(),
        Some("vue") | Some("vue3") | Some("vite") => "dist".to_string(),
        Some("nuxtjs") => ".output/public".to_string(),
        Some("svelte") | Some("sveltekit") => "build".to_string(),
        Some("gatsby") => "public".to_string(),
        Some("astro") => "dist".to_string(),
        Some("remix") => "public".to_string(),
        _ => "dist".to_string(),
    }
}

/// Run the build command for the project
async fn run_build_command(project_path: &str, config: &DeploymentConfig) -> Result<(), String> {
    use crate::commands::monorepo::get_volta_wrapped_command;
    use crate::utils::path_resolver;
    use std::path::Path;
    use std::process::Stdio;

    // Use custom build command if set, otherwise default to "npm run build"
    let build_cmd = config.build_command.as_deref().unwrap_or("npm run build");

    // Parse the build command
    let parts: Vec<&str> = build_cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty build command".to_string());
    }

    let base_cmd = parts[0];
    let base_args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    // Use version manager wrapping (Volta/Corepack) for proper Node.js/npm resolution
    let project_path_buf = Path::new(project_path);
    let (final_cmd, final_args) = get_volta_wrapped_command(project_path_buf, base_cmd, base_args);

    println!(
        "[deploy] Running build command: {} {:?} in {}",
        final_cmd, final_args, project_path
    );

    // Use path_resolver to create command with proper PATH for macOS GUI apps
    let mut command = path_resolver::create_command(&final_cmd);
    command.args(&final_args);
    command.current_dir(project_path);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = tokio::process::Command::from(command)
        .output()
        .await
        .map_err(|e| format!("Failed to run build command '{}': {}", build_cmd, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Build command '{}' failed:\n{}\n{}",
            build_cmd, stdout, stderr
        ));
    }

    Ok(())
}

/// Collect all files from a directory for upload
fn collect_files_for_upload(
    build_path: &std::path::Path,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    use std::fs;

    let mut files = Vec::new();

    fn collect_recursive(
        base: &std::path::Path,
        current: &std::path::Path,
        files: &mut Vec<(String, Vec<u8>)>,
    ) -> Result<(), String> {
        for entry in
            fs::read_dir(current).map_err(|e| format!("Failed to read directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                collect_recursive(base, &path, files)?;
            } else {
                let relative = path
                    .strip_prefix(base)
                    .map_err(|e| format!("Failed to get relative path: {}", e))?
                    .to_string_lossy()
                    .to_string();
                let content = fs::read(&path)
                    .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;
                files.push((relative, content));
            }
        }
        Ok(())
    }

    collect_recursive(build_path, build_path, &mut files)?;
    Ok(files)
}

/// Calculate SHA1 hash of content (hex string)
fn calculate_sha1(content: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

/// Deploy to GitHub Pages by pushing to gh-pages branch
async fn deploy_to_github_pages(
    app: &AppHandle,
    deployment_id: &str,
    project_path: &str,
    _config: &DeploymentConfig,
    build_path: &std::path::Path,
) -> Result<(String, String), String> {
    use crate::utils::path_resolver;
    use std::process::Stdio;

    // Emit deploying status
    let _ = app.emit(
        "deployment:status",
        DeploymentStatusEvent {
            deployment_id: deployment_id.to_string(),
            status: DeploymentStatus::Deploying,
            url: None,
            error_message: None,
        },
    );

    // Get git remote URL to determine GitHub Pages URL
    // Use path_resolver for proper PATH handling in macOS GUI apps
    let remote_output = path_resolver::create_async_command("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to get git remote: {}", e))?;

    if !remote_output.status.success() {
        return Err("No git remote 'origin' found. Please configure git remote first.".to_string());
    }

    let remote_url = String::from_utf8_lossy(&remote_output.stdout)
        .trim()
        .to_string();

    // Parse GitHub username and repo from remote URL
    let (username, repo) = parse_github_remote(&remote_url)?;

    // Create a temporary directory for gh-pages branch
    let temp_dir =
        std::env::temp_dir().join(format!("specforge-gh-pages-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    // Clone the gh-pages branch (or create it)
    let clone_result = path_resolver::create_async_command("git")
        .args([
            "clone",
            "--branch",
            "gh-pages",
            "--single-branch",
            "--depth",
            "1",
            &remote_url,
            ".",
        ])
        .current_dir(&temp_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    let is_new_branch = match clone_result {
        Ok(output) if output.status.success() => false,
        _ => {
            // gh-pages branch doesn't exist, initialize a new orphan branch
            path_resolver::create_async_command("git")
                .args(["init"])
                .current_dir(&temp_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("Failed to init git: {}", e))?;

            path_resolver::create_async_command("git")
                .args(["checkout", "--orphan", "gh-pages"])
                .current_dir(&temp_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("Failed to create orphan branch: {}", e))?;

            path_resolver::create_async_command("git")
                .args(["remote", "add", "origin", &remote_url])
                .current_dir(&temp_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("Failed to add remote: {}", e))?;

            true
        }
    };

    // Clear existing files (except .git)
    for entry in
        std::fs::read_dir(&temp_dir).map_err(|e| format!("Failed to read temp dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.file_name().map(|n| n != ".git").unwrap_or(false) {
            if path.is_dir() {
                std::fs::remove_dir_all(&path).ok();
            } else {
                std::fs::remove_file(&path).ok();
            }
        }
    }

    // Copy build files to temp directory
    copy_dir_contents(build_path, &temp_dir)?;

    // Add .nojekyll file to prevent Jekyll processing
    std::fs::write(temp_dir.join(".nojekyll"), "")
        .map_err(|e| format!("Failed to create .nojekyll: {}", e))?;

    // Git add all files
    path_resolver::create_async_command("git")
        .args(["add", "-A"])
        .current_dir(&temp_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to git add: {}", e))?;

    // Git commit
    let commit_msg = format!(
        "Deploy from SpecForge - {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    let commit_output = path_resolver::create_async_command("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(&temp_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to git commit: {}", e))?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        // If no changes to commit, that's OK
        if !stderr.contains("nothing to commit") {
            return Err(format!("Git commit failed: {}", stderr));
        }
    }

    // Git push with timeout
    let push_args = if is_new_branch {
        vec!["push", "-u", "origin", "gh-pages"]
    } else {
        vec!["push", "origin", "gh-pages"]
    };

    println!("[Deploy] Starting git push to gh-pages branch...");

    let mut push_cmd = path_resolver::create_async_command("git");
    push_cmd
        .args(&push_args)
        .current_dir(&temp_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Disable interactive prompts
        .env("GIT_TERMINAL_PROMPT", "0")
        .env(
            "GIT_SSH_COMMAND",
            "ssh -o BatchMode=yes -o StrictHostKeyChecking=no",
        );
    let push_future = push_cmd.output();

    // Add timeout of 60 seconds
    let push_output = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        push_future
    )
    .await
    .map_err(|_| {
        let _ = std::fs::remove_dir_all(&temp_dir);
        "Git push timed out after 60 seconds. Please check your git credentials and network connection.".to_string()
    })?
    .map_err(|e| {
        let _ = std::fs::remove_dir_all(&temp_dir);
        format!("Failed to git push: {}", e)
    })?;

    println!(
        "[Deploy] Git push completed with status: {}",
        push_output.status
    );

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        let stdout = String::from_utf8_lossy(&push_output.stdout);
        println!(
            "[Deploy] Git push failed - stdout: {}, stderr: {}",
            stdout, stderr
        );
        return Err(format!(
            "Git push failed: {}. Make sure you have push access and valid credentials.",
            stderr
        ));
    }

    println!("[Deploy] GitHub Pages deployment successful!");

    // Construct GitHub Pages URL
    let pages_url = format!("https://{}.github.io/{}/", username, repo);

    Ok((pages_url, deployment_id.to_string()))
}

/// Parse GitHub username and repo from remote URL
fn parse_github_remote(url: &str) -> Result<(String, String), String> {
    // Handle SSH format: git@github.com:username/repo.git
    if url.starts_with("git@github.com:") {
        let path = url.strip_prefix("git@github.com:").unwrap();
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    // Handle HTTPS format, potentially with embedded username
    if url.contains("github.com") {
        let mut clean_url = url.to_string();
        if let Some(schema_end) = clean_url.find("://") {
            if let Some(at_pos) = clean_url[(schema_end + 3)..].find('@') {
                let end_of_user = (schema_end + 3) + at_pos + 1;
                clean_url.replace_range((schema_end + 3)..end_of_user, "");
            }
        }

        let parsed_url = url::Url::parse(&clean_url).map_err(|e| format!("Invalid URL: {}", e))?;
        let path = parsed_url.path().trim_start_matches('/');
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    Err("Could not parse GitHub remote URL. Expected format: git@github.com:user/repo.git or https://github.com/user/repo.git".to_string())
}

/// Copy directory contents recursively
fn copy_dir_contents(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    use std::fs;

    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read source dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)
                .map_err(|e| format!("Failed to create dir {}: {}", dst_path.display(), e))?;
            copy_dir_contents(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy file {}: {}", src_path.display(), e))?;
        }
    }
    Ok(())
}

/// Deploy to Netlify using file digest API
async fn deploy_to_netlify(
    app: &AppHandle,
    deployment_id: &str,
    access_token: &str,
    config: &DeploymentConfig,
    build_path: &std::path::Path,
) -> Result<DeployResult, String> {
    let client = reqwest::Client::new();

    // Step 1: Get site_id - use saved one or create new
    let site_id = if let Some(existing_id) = &config.netlify_site_id {
        // Verify the site still exists
        let check_url = format!("{}/{}", NETLIFY_SITES_URL, existing_id);
        let check = client
            .get(&check_url)
            .bearer_auth(access_token)
            .send()
            .await;
        if check.map(|r| r.status().is_success()).unwrap_or(false) {
            existing_id.clone()
        } else {
            // Site no longer exists, create new one
            get_or_create_netlify_site(&client, access_token, config).await?
        }
    } else {
        get_or_create_netlify_site(&client, access_token, config).await?
    };

    // Save site_id to config for future deployments
    save_netlify_site_id(app, &config.project_id, &site_id)?;

    // Emit deploying status
    let _ = app.emit(
        "deployment:status",
        DeploymentStatusEvent {
            deployment_id: deployment_id.to_string(),
            status: DeploymentStatus::Deploying,
            url: None,
            error_message: None,
        },
    );

    // Step 2: Collect files and create digest map
    let files = collect_files_for_upload(build_path)?;

    // Build file digest map (path -> sha1)
    let mut file_digests = std::collections::HashMap::new();
    for (path, content) in &files {
        let sha = calculate_sha1(content);
        // Netlify expects paths starting with /
        file_digests.insert(format!("/{}", path), sha);
    }

    // Step 3: Create a deploy with file digests
    let deploy_url = format!("{}/{}/deploys", NETLIFY_SITES_URL, site_id);
    let payload = serde_json::json!({
        "files": file_digests,
    });

    let response = client
        .post(&deploy_url)
        .bearer_auth(access_token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Netlify deploy request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Netlify deployment failed: {}", error_text));
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Netlify deploy response: {}", e))?;

    let netlify_deploy_id = result["id"].as_str().unwrap_or("").to_string();

    // Step 4: Upload required files
    if let Some(required) = result["required"].as_array() {
        for sha in required {
            if let Some(sha_str) = sha.as_str() {
                // Find file with matching SHA
                for (path, content) in &files {
                    if calculate_sha1(content) == sha_str {
                        upload_file_to_netlify(
                            &client,
                            access_token,
                            &netlify_deploy_id,
                            &format!("/{}", path),
                            content,
                        )
                        .await?;
                        break;
                    }
                }
            }
        }
    }

    // Step 5: Poll for deployment status and get extended info
    let result = poll_netlify_deployment(
        app,
        deployment_id,
        access_token,
        &site_id,
        &netlify_deploy_id,
    )
    .await?;

    Ok(result)
}

/// Upload a single file to Netlify deploy
async fn upload_file_to_netlify(
    client: &reqwest::Client,
    access_token: &str,
    deploy_id: &str,
    file_path: &str,
    content: &[u8],
) -> Result<(), String> {
    let url = format!(
        "https://api.netlify.com/api/v1/deploys/{}/files{}",
        deploy_id, file_path
    );

    let response = client
        .put(&url)
        .bearer_auth(access_token)
        .header("Content-Type", "application/octet-stream")
        .body(content.to_vec())
        .send()
        .await
        .map_err(|e| format!("Failed to upload file {}: {}", file_path, e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to upload {}: {}", file_path, error_text));
    }

    Ok(())
}

/// Get or create a Netlify site for the project
async fn get_or_create_netlify_site(
    client: &reqwest::Client,
    access_token: &str,
    config: &DeploymentConfig,
) -> Result<String, String> {
    // Use custom site name if provided, otherwise sanitize project_id
    let site_name = config
        .netlify_site_name
        .as_ref()
        .map(|n| sanitize_site_name(n))
        .unwrap_or_else(|| sanitize_site_name(&config.project_id));

    // First, try to find an existing site with matching name
    let response = client
        .get(NETLIFY_SITES_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to list Netlify sites: {}", e))?;

    if response.status().is_success() {
        let sites: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Netlify sites: {}", e))?;

        // Look for a site matching the site name
        if let Some(site) = sites
            .iter()
            .find(|s| s["name"].as_str().map(|n| n == site_name).unwrap_or(false))
        {
            if let Some(site_id) = site["id"].as_str() {
                return Ok(site_id.to_string());
            }
        }
    }

    // Create a new site if not found
    let create_payload = serde_json::json!({
        "name": site_name,
    });

    let create_response = client
        .post(NETLIFY_SITES_URL)
        .bearer_auth(access_token)
        .json(&create_payload)
        .send()
        .await
        .map_err(|e| format!("Failed to create Netlify site: {}", e))?;

    if !create_response.status().is_success() {
        let error_text = create_response.text().await.unwrap_or_default();
        return Err(format!("Failed to create Netlify site: {}", error_text));
    }

    let site: serde_json::Value = create_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Netlify site response: {}", e))?;

    site["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No site ID in Netlify response".to_string())
}

/// Sanitize project name for Netlify site name (lowercase, alphanumeric, hyphens only)
fn sanitize_site_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Poll Netlify deployment status and return extended deployment info
async fn poll_netlify_deployment(
    app: &AppHandle,
    deployment_id: &str,
    access_token: &str,
    site_id: &str,
    netlify_deploy_id: &str,
) -> Result<DeployResult, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/{}/deploys/{}",
        NETLIFY_SITES_URL, site_id, netlify_deploy_id
    );

    for _ in 0..60 {
        // Max 5 minutes polling
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let response = client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| format!("Failed to check Netlify deployment status: {}", e))?;

        if !response.status().is_success() {
            continue;
        }

        let status: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Netlify status response: {}", e))?;

        match status["state"].as_str() {
            Some("ready") => {
                // Extract all available info from the response
                let deploy_url = status["ssl_url"]
                    .as_str()
                    .or_else(|| status["url"].as_str())
                    .unwrap_or("")
                    .to_string();

                let admin_url = status["admin_url"].as_str().map(|s| s.to_string());
                let deploy_time = status["deploy_time"].as_u64();
                let site_name = status["name"].as_str().map(|s| s.to_string());
                let preview_url = status["deploy_ssl_url"]
                    .as_str()
                    .or_else(|| status["deploy_url"].as_str())
                    .map(|s| s.to_string());

                return Ok(DeployResult {
                    url: deploy_url,
                    deploy_id: netlify_deploy_id.to_string(),
                    admin_url,
                    deploy_time,
                    site_name,
                    preview_url,
                    branch: None, // Netlify doesn't return branch in deploy status
                });
            }
            Some("error") => {
                let error = status["error_message"]
                    .as_str()
                    .unwrap_or("Deployment failed")
                    .to_string();
                return Err(error);
            }
            Some("building") => {
                let _ = app.emit(
                    "deployment:status",
                    DeploymentStatusEvent {
                        deployment_id: deployment_id.to_string(),
                        status: DeploymentStatus::Building,
                        url: None,
                        error_message: None,
                    },
                );
            }
            Some("uploading") | Some("uploaded") | Some("processing") => {
                let _ = app.emit(
                    "deployment:status",
                    DeploymentStatusEvent {
                        deployment_id: deployment_id.to_string(),
                        status: DeploymentStatus::Deploying,
                        url: None,
                        error_message: None,
                    },
                );
            }
            _ => {}
        }
    }

    Err("Netlify deployment timed out".to_string())
}

/// Get deployment history for a project
#[tauri::command]
pub async fn get_deployment_history(
    app: AppHandle,
    project_id: String,
) -> Result<Vec<Deployment>, String> {
    get_deployments_from_store(&app, &project_id)
}

/// Delete a single deployment from history
#[tauri::command]
pub async fn delete_deployment_history_item(
    app: AppHandle,
    _project_id: String,
    deployment_id: String,
) -> Result<(), String> {
    let repo = get_deploy_repo(&app);
    repo.delete_deployment(&deployment_id)?;
    Ok(())
}

/// Clear all deployment history for a project
#[tauri::command]
pub async fn clear_deployment_history(app: AppHandle, project_id: String) -> Result<(), String> {
    let repo = get_deploy_repo(&app);
    repo.clear_deployments(&project_id)?;
    Ok(())
}

/// Get deployment config for a project
#[tauri::command]
pub async fn get_deployment_config(
    app: AppHandle,
    project_id: String,
) -> Result<Option<DeploymentConfig>, String> {
    get_config_from_store(&app, &project_id)
}

/// Save deployment config for a project
#[tauri::command]
pub async fn save_deployment_config(
    app: AppHandle,
    config: DeploymentConfig,
) -> Result<(), String> {
    save_config_to_store(&app, &config)
}

/// Delete deployment config for a project
#[tauri::command]
pub async fn delete_deployment_config(
    app: AppHandle,
    project_id: String,
) -> Result<bool, String> {
    println!("[delete_deployment_config] project_id={}", project_id);

    let db = get_db(&app);
    let repo = get_deploy_repo(&app);
    let result = repo.delete_config(&project_id);

    // Force WAL checkpoint to ensure deletion is persisted
    if result.is_ok() {
        if let Err(e) = db.with_connection(|conn| {
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|e| format!("WAL checkpoint failed: {}", e))
        }) {
            println!("[delete_deployment_config] WAL checkpoint warning: {}", e);
        }
        println!("[delete_deployment_config] SUCCESS for project_id={}", project_id);
    }

    result
}

/// Detect framework from project path
#[tauri::command]
pub async fn detect_framework(project_path: String) -> Result<Option<String>, String> {
    let package_json_path = std::path::Path::new(&project_path).join("package.json");

    if !package_json_path.exists() {
        return Ok(Some("static".to_string()));
    }

    let content = std::fs::read_to_string(&package_json_path)
        .map_err(|e| format!("Failed to read package.json: {}", e))?;

    let package: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse package.json: {}", e))?;

    let deps = package["dependencies"].as_object();
    let dev_deps = package["devDependencies"].as_object();

    let has_dep = |name: &str| -> bool {
        deps.map(|d| d.contains_key(name)).unwrap_or(false)
            || dev_deps.map(|d| d.contains_key(name)).unwrap_or(false)
    };

    // Detection order matters - more specific frameworks first
    let framework = if has_dep("next") {
        "nextjs"
    } else if has_dep("nuxt") {
        "nuxtjs"
    } else if has_dep("@remix-run/react") {
        "remix"
    } else if has_dep("gatsby") {
        "gatsby"
    } else if has_dep("@sveltejs/kit") {
        "sveltekit"
    } else if has_dep("astro") {
        "astro"
    } else if has_dep("vite") {
        "vite"
    } else if has_dep("react") {
        "create-react-app"
    } else if has_dep("vue") {
        "vue"
    } else {
        "static"
    };

    Ok(Some(framework.to_string()))
}

/// Redeploy using last deployment config
#[tauri::command]
pub async fn redeploy(
    app: AppHandle,
    project_id: String,
    project_path: String,
) -> Result<Deployment, String> {
    // Get last deployment config
    let config = get_deployment_config(app.clone(), project_id.clone())
        .await?
        .ok_or("No previous deployment config found")?;

    // Start new deployment with same config
    start_deployment(app, project_id, project_path, config).await
}

// ============================================================================
// Multi Deploy Accounts Commands (016-multi-deploy-accounts)
// ============================================================================

/// T009: Get all deploy accounts (sanitized - no tokens)
#[tauri::command]
pub async fn get_deploy_accounts(app: AppHandle) -> Result<Vec<DeployAccount>, String> {
    let accounts = get_accounts_from_store(&app)?;
    Ok(accounts.into_iter().map(|a| a.sanitized()).collect())
}

/// T010: Get accounts filtered by platform
#[tauri::command]
pub async fn get_accounts_by_platform(
    app: AppHandle,
    platform: PlatformType,
) -> Result<Vec<DeployAccount>, String> {
    let accounts = get_accounts_from_store(&app)?;
    Ok(accounts
        .into_iter()
        .filter(|a| a.platform == platform)
        .map(|a| a.sanitized())
        .collect())
}

/// T015: Add a new deploy account via OAuth
#[tauri::command]
pub async fn add_deploy_account(
    app: AppHandle,
    platform: PlatformType,
) -> Result<OAuthFlowResult, String> {
    use uuid::Uuid;

    // Check max accounts limit
    let accounts = get_accounts_from_store(&app)?;
    let platform_count = accounts.iter().filter(|a| a.platform == platform).count();
    if platform_count >= MAX_ACCOUNTS_PER_PLATFORM {
        return Ok(OAuthFlowResult {
            success: false,
            platform: None,
            error: Some(format!(
                "Maximum of {} accounts per platform reached",
                MAX_ACCOUNTS_PER_PLATFORM
            )),
        });
    }

    let oauth_config = match get_oauth_client_config(&platform) {
        Ok(config) => config,
        Err(error) => {
            return Ok(OAuthFlowResult {
                success: false,
                platform: None,
                error: Some(error),
            });
        }
    };

    // Generate state for CSRF protection
    let state = Uuid::new_v4().to_string();
    let state_clone = state.clone();

    // Channel to receive the callback URL
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    // Start local OAuth server with callback on fixed port
    let config = tauri_plugin_oauth::OauthConfig {
        ports: Some(vec![8766, 8767, 8768]), // Try these ports in order
        response: Some(OAUTH_SUCCESS_HTML.into()),
    };
    let port = tauri_plugin_oauth::start_with_config(config, move |url| {
        let tx = tx.clone();
        tauri::async_runtime::spawn(async move {
            if let Some(sender) = tx.lock().await.take() {
                let _ = sender.send(url);
            }
        });
    })
    .map_err(|e| format!("Failed to start OAuth server: {}", e))?;

    let redirect_uri = format!("http://localhost:{}/callback", port);

    // Build authorization URL based on platform
    let auth_url = match &oauth_config {
        OAuthClientConfig::Netlify { client_id } => {
            build_netlify_auth_url(client_id, &redirect_uri, &state)
        }
    };

    // Open browser for authorization
    if let Err(e) = opener::open_browser(&auth_url) {
        let _ = tauri_plugin_oauth::cancel(port);
        return Ok(OAuthFlowResult {
            success: false,
            platform: None,
            error: Some(format!("Failed to open browser: {}", e)),
        });
    }

    // Wait for callback with timeout (60 seconds)
    let callback_result = tokio::time::timeout(std::time::Duration::from_secs(60), rx).await;

    // Cancel the OAuth server
    let _ = tauri_plugin_oauth::cancel(port);

    let callback_url = match callback_result {
        Ok(Ok(url)) => url,
        Ok(Err(_)) => {
            return Ok(OAuthFlowResult {
                success: false,
                platform: None,
                error: Some("OAuth callback channel closed".to_string()),
            });
        }
        Err(_) => {
            return Ok(OAuthFlowResult {
                success: false,
                platform: None,
                error: Some("OAuth flow timed out".to_string()),
            });
        }
    };

    // Parse callback URL and exchange for token
    let connected_platform = match oauth_config {
        OAuthClientConfig::Netlify { .. } => {
            extract_netlify_token(&callback_url, &state_clone).await?
        }
    };

    // Convert to DeployAccount
    let new_account = DeployAccount::from_connected_platform(connected_platform.clone());

    // Check for duplicate account (same platform + platform_user_id)
    let accounts = get_accounts_from_store(&app)?;
    if accounts
        .iter()
        .any(|a| a.platform == platform && a.platform_user_id == new_account.platform_user_id)
    {
        return Ok(OAuthFlowResult {
            success: false,
            platform: None,
            error: Some("This account is already connected".to_string()),
        });
    }

    // Add new account using repository
    save_account_to_store(&app, &new_account)?;

    Ok(OAuthFlowResult {
        success: true,
        platform: Some(connected_platform.sanitized()),
        error: None,
    })
}

/// T016: Remove a deploy account
#[tauri::command]
pub async fn remove_deploy_account(
    app: AppHandle,
    account_id: String,
    force: Option<bool>,
) -> Result<RemoveAccountResult, String> {
    let force = force.unwrap_or(false);

    // Check for affected projects
    let affected_projects = find_projects_using_account(&app, &account_id)?;

    if !affected_projects.is_empty() && !force {
        return Ok(RemoveAccountResult {
            success: false,
            affected_projects,
        });
    }

    // If force, clear the account from all configs
    if force && !affected_projects.is_empty() {
        clear_account_from_configs(&app, &account_id)?;
    }

    // T049: Clear default preference if this account was default
    let mut prefs = get_preferences_from_store(&app)?;
    prefs.clear_if_matches(&account_id);
    save_preferences_to_store(&app, &prefs)?;

    // Remove the account using repository
    let repo = get_deploy_repo(&app);
    repo.delete_account(&account_id)?;

    Ok(RemoveAccountResult {
        success: true,
        affected_projects,
    })
}

/// T022: Update deploy account (display name)
#[tauri::command]
pub async fn update_deploy_account(
    app: AppHandle,
    account_id: String,
    display_name: Option<String>,
) -> Result<DeployAccount, String> {
    let repo = get_deploy_repo(&app);

    let mut account = repo.get_account(&account_id)?
        .ok_or("Account not found")?;

    account.display_name = display_name;
    repo.save_account(&account)?;

    Ok(account.sanitized())
}

/// T028: Bind a project to a specific deploy account
#[tauri::command]
pub async fn bind_project_account(
    app: AppHandle,
    project_id: String,
    account_id: String,
) -> Result<DeploymentConfig, String> {
    let repo = get_deploy_repo(&app);

    // Verify account exists
    let account = repo.get_account(&account_id)?.ok_or("Account not found")?;

    // Get or create deployment config
    let mut config = repo.get_config(&project_id)?.unwrap_or(DeploymentConfig {
        project_id: project_id.clone(),
        platform: account.platform.clone(),
        account_id: None,
        environment: crate::models::deploy::DeploymentEnvironment::Production,
        framework_preset: None,
        env_variables: Vec::new(),
        root_directory: None,
        install_command: None,
        build_command: None,
        output_directory: None,
        netlify_site_id: None,
        netlify_site_name: None,
        cloudflare_account_id: None,
        cloudflare_project_name: None,
    });

    // Verify platform matches
    if config.platform != account.platform {
        return Err("Account platform does not match project's deploy platform".to_string());
    }

    config.account_id = Some(account_id);
    repo.save_config(&config)?;
    Ok(config)
}

/// T029: Unbind a project from its deploy account
#[tauri::command]
pub async fn unbind_project_account(
    app: AppHandle,
    project_id: String,
) -> Result<DeploymentConfig, String> {
    let repo = get_deploy_repo(&app);

    let mut config = repo.get_config(&project_id)?.ok_or("Deployment config not found")?;

    config.account_id = None;
    repo.save_config(&config)?;
    Ok(config)
}

/// T030: Get the account bound to a project
#[tauri::command]
pub async fn get_project_binding(
    app: AppHandle,
    project_id: String,
) -> Result<Option<DeployAccount>, String> {
    let repo = get_deploy_repo(&app);

    let config = match repo.get_config(&project_id)? {
        Some(c) => c,
        None => return Ok(None),
    };

    let account_id = match &config.account_id {
        Some(id) => id,
        None => return Ok(None),
    };

    Ok(repo.get_account(account_id)?.map(|a| a.sanitized()))
}

/// T042: Get deploy preferences
#[tauri::command]
pub async fn get_deploy_preferences(app: AppHandle) -> Result<DeployPreferences, String> {
    get_preferences_from_store(&app)
}

/// T043: Set default account for a platform
#[tauri::command]
pub async fn set_default_account(
    app: AppHandle,
    platform: PlatformType,
    account_id: Option<String>,
) -> Result<DeployPreferences, String> {
    let repo = get_deploy_repo(&app);

    // If setting a default, verify account exists and matches platform
    if let Some(ref id) = account_id {
        let account = repo.get_account(id)?.ok_or("Account not found")?;
        if account.platform != platform {
            return Err("Account platform does not match".to_string());
        }
    }

    let mut prefs = repo.get_preferences()?;
    prefs.set_default_account_id(&platform, account_id);
    repo.save_preferences(&prefs)?;

    Ok(prefs)
}

// ============================================================================
// GitHub Pages Workflow Generation (Phase 2)
// ============================================================================

/// GitHub Actions workflow template for GitHub Pages deployment
const GITHUB_PAGES_WORKFLOW_TEMPLATE: &str = r#"# Deploy to GitHub Pages
# Generated by SpecForge - https://github.com/runkids/packageflow
#
# This workflow builds your site and deploys it to GitHub Pages.
# Make sure GitHub Pages is enabled in your repository settings:
# Settings → Pages → Source: "GitHub Actions"

name: Deploy to GitHub Pages

on:
  # Runs on pushes targeting the default branch
  push:
    branches: ["{{BRANCH}}"]

  # Allows you to run this workflow manually from the Actions tab
  workflow_dispatch:

# Sets permissions of the GITHUB_TOKEN to allow deployment to GitHub Pages
permissions:
  contents: read
  pages: write
  id-token: write

# Allow only one concurrent deployment, skipping runs queued between the run in-progress and latest queued.
# However, do NOT cancel in-progress runs as we want to allow these production deployments to complete.
concurrency:
  group: "pages"
  cancel-in-progress: false

jobs:
  # Build job
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: "20"

{{BUN_SETUP}}

      - name: Enable Corepack
        run: corepack enable

      - name: Install dependencies
        run: {{INSTALL_COMMAND}}

      - name: Build
        run: {{BUILD_COMMAND}}

      - name: Setup Pages
        uses: actions/configure-pages@v5

      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: "{{OUTPUT_DIRECTORY}}"

  # Deployment job
  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    needs: build
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
"#;

/// Detect package manager from project
fn detect_package_manager(project_path: &str) -> (&'static str, &'static str) {
    let path = std::path::Path::new(project_path);

    // Check for lock files in order of preference
    if path.join("pnpm-lock.yaml").exists() {
        ("pnpm", "pnpm install --frozen-lockfile")
    } else if path.join("yarn.lock").exists() {
        ("yarn", "yarn install --frozen-lockfile")
    } else if path.join("bun.lockb").exists() {
        ("bun", "bun install --frozen-lockfile")
    } else {
        ("npm", "npm ci")
    }
}

/// Detect default branch from git
async fn detect_default_branch(project_path: &str) -> String {
    use crate::utils::path_resolver;
    use std::process::Stdio;

    // Try to get the default branch from remote
    let output = path_resolver::create_async_command("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .current_dir(project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    if let Ok(output) = output {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout)
                .trim()
                .replace("refs/remotes/origin/", "");
            if !branch.is_empty() {
                return branch;
            }
        }
    }

    // Fall back to checking current branch
    let output = path_resolver::create_async_command("git")
        .args(["branch", "--show-current"])
        .current_dir(project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    if let Ok(output) = output {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !branch.is_empty() {
                return branch;
            }
        }
    }

    // Default to "main"
    "main".to_string()
}

/// Generate GitHub Actions workflow content
fn generate_workflow_content(
    branch: &str,
    bun_setup: &str,
    install_command: &str,
    build_command: &str,
    output_directory: &str,
) -> String {
    GITHUB_PAGES_WORKFLOW_TEMPLATE
        .replace("{{BRANCH}}", branch)
        .replace("{{BUN_SETUP}}", bun_setup)
        .replace("{{INSTALL_COMMAND}}", install_command)
        .replace("{{BUILD_COMMAND}}", build_command)
        .replace("{{OUTPUT_DIRECTORY}}", output_directory)
}

#[tauri::command]
pub async fn generate_github_actions_workflow(
    project_path: String,
    config: DeploymentConfig,
) -> Result<GitHubWorkflowResult, String> {
    use crate::utils::path_resolver;
    use std::fs;
    use std::process::Stdio;

    // --- Get GitHub repo info for URL generation ---
    let (username_str, repo_str) = match path_resolver::create_async_command("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(&project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
    {
        Ok(remote_output) if remote_output.status.success() => {
            let remote_url = String::from_utf8_lossy(&remote_output.stdout)
                .trim()
                .to_string();
            parse_github_remote(&remote_url)
                .unwrap_or(("<username>".to_string(), "<repo>".to_string()))
        }
        _ => ("<username>".to_string(), "<repo>".to_string()),
    };
    let username = if username_str == "<username>" {
        None
    } else {
        Some(username_str.clone())
    };
    let repo = if repo_str == "<repo>" {
        None
    } else {
        Some(repo_str.clone())
    };

    // Detect package manager
    let (package_manager, install_command) = detect_package_manager(&project_path);

    // Detect default branch
    let branch = detect_default_branch(&project_path).await;

    // Determine build command and output directory
    let build_command = config
        .build_command
        .as_deref()
        .unwrap_or("npm run build")
        .to_string();

    let output_directory = config
        .output_directory
        .clone()
        .unwrap_or_else(|| get_build_output_dir(config.framework_preset.as_deref()));

    // Generate workflow content
    let workflow_content = generate_workflow_content(
        &branch,
        package_manager,
        install_command,
        &build_command,
        &output_directory,
    );

    // Create .github/workflows directory
    let workflows_dir = std::path::Path::new(&project_path)
        .join(".github")
        .join("workflows");
    fs::create_dir_all(&workflows_dir)
        .map_err(|e| format!("Failed to create workflows directory: {}", e))?;

    // Write workflow file
    let workflow_path = workflows_dir.join("deploy-pages.yml");
    fs::write(&workflow_path, &workflow_content)
        .map_err(|e| format!("Failed to write workflow file: {}", e))?;

    // Generate setup instructions
    let setup_instructions = vec![
        "1. Commit and push the generated workflow file to your repository.".to_string(),
        format!("2. Go to your repository Settings → Pages"),
        "3. Under 'Build and deployment', set Source to 'GitHub Actions' (not 'Deploy from a branch').".to_string(),
        "   - Important: If GitHub Pages is not yet enabled or configured correctly, the deployment might fail with a 404 error when 'configure-pages' tries to access the site.".to_string(),
        format!("4. Push changes to the '{}' branch to trigger a deployment", branch),
        format!("5. Your site will be available at https://{}.github.io/{}/", username_str, repo_str),
    ];

    // Return relative path for display
    let relative_path = format!(".github/workflows/deploy-pages.yml");

    Ok(GitHubWorkflowResult {
        success: true,
        workflow_path: relative_path,
        setup_instructions,
        username,
        repo,
    })
}

// ============================================================================
// Cloudflare Pages Integration (Phase 3)
// ============================================================================

/// Validate Cloudflare API token and get account info
#[tauri::command]
pub async fn validate_cloudflare_token(
    api_token: String,
) -> Result<CloudflareValidationResult, String> {
    let client = reqwest::Client::new();

    // First, verify the token is valid
    let verify_response = client
        .get(CLOUDFLARE_VERIFY_URL)
        .bearer_auth(&api_token)
        .send()
        .await
        .map_err(|e| format!("Failed to verify token: {}", e))?;

    if !verify_response.status().is_success() {
        return Ok(CloudflareValidationResult {
            valid: false,
            account_id: None,
            account_name: None,
            error: Some("Invalid API token".to_string()),
        });
    }

    let verify_data: serde_json::Value = verify_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse verify response: {}", e))?;

    if !verify_data["success"].as_bool().unwrap_or(false) {
        return Ok(CloudflareValidationResult {
            valid: false,
            account_id: None,
            account_name: None,
            error: Some("Token verification failed".to_string()),
        });
    }

    // Get accounts list to find the account ID
    let accounts_url = format!("{}/accounts", CLOUDFLARE_API_BASE);
    let accounts_response = client
        .get(&accounts_url)
        .bearer_auth(&api_token)
        .send()
        .await
        .map_err(|e| format!("Failed to get accounts: {}", e))?;

    if !accounts_response.status().is_success() {
        return Ok(CloudflareValidationResult {
            valid: true, // Token is valid but can't get accounts
            account_id: None,
            account_name: None,
            error: Some("Could not retrieve account information".to_string()),
        });
    }

    let accounts_data: serde_json::Value = accounts_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse accounts response: {}", e))?;

    // Get the first account (most users have one account)
    if let Some(accounts) = accounts_data["result"].as_array() {
        if let Some(account) = accounts.first() {
            return Ok(CloudflareValidationResult {
                valid: true,
                account_id: account["id"].as_str().map(|s| s.to_string()),
                account_name: account["name"].as_str().map(|s| s.to_string()),
                error: None,
            });
        }
    }

    Ok(CloudflareValidationResult {
        valid: true,
        account_id: None,
        account_name: None,
        error: Some("No accounts found".to_string()),
    })
}

/// Add Cloudflare Pages account via API token
#[tauri::command]
pub async fn add_cloudflare_account(
    app: AppHandle,
    api_token: String,
    display_name: Option<String>,
) -> Result<DeployAccount, String> {
    // Validate token first
    let validation = validate_cloudflare_token(api_token.clone()).await?;

    if !validation.valid {
        return Err(validation
            .error
            .unwrap_or_else(|| "Invalid token".to_string()));
    }

    let account_id = validation
        .account_id
        .ok_or("Could not retrieve Cloudflare account ID")?;
    let account_name = validation
        .account_name
        .unwrap_or_else(|| "Cloudflare Account".to_string());

    // Check for duplicate
    let accounts = get_accounts_from_store(&app)?;
    if accounts
        .iter()
        .any(|a| a.platform == PlatformType::CloudflarePages && a.platform_user_id == account_id)
    {
        return Err("This Cloudflare account is already connected".to_string());
    }

    // Create new account
    let new_account = DeployAccount {
        id: uuid::Uuid::new_v4().to_string(),
        platform: PlatformType::CloudflarePages,
        platform_user_id: account_id,
        username: account_name,
        display_name,
        avatar_url: None,
        access_token: api_token,
        connected_at: chrono::Utc::now(),
        expires_at: None, // API tokens don't expire
    };

    // Save account using repository
    save_account_to_store(&app, &new_account)?;

    Ok(new_account.sanitized())
}

// Legacy Cloudflare deployment functions removed (2024-12)
// Now using services::deploy::cloudflare::CloudflareProvider
// Removed: deploy_to_cloudflare_pages, get_mime_type, poll_cloudflare_deployment

/// Check if an account is in use by any projects
#[tauri::command]
pub async fn check_account_usage(
    app: AppHandle,
    account_id: String,
) -> Result<CheckAccountResult, String> {
    let affected_projects = find_projects_using_account(&app, &account_id)?;
    Ok(CheckAccountResult {
        in_use: !affected_projects.is_empty(),
        affected_projects,
    })
}

// ============================================================================
// Secure Backup Commands (Token Encryption)
// ============================================================================

/// Result of backup export operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupExportResult {
    /// Encrypted backup data (can be saved to file)
    pub encrypted_data: EncryptedData,
    /// Number of accounts included in backup
    pub account_count: usize,
}

/// Result of backup import operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupImportResult {
    pub success: bool,
    /// Number of accounts restored
    pub accounts_restored: usize,
    /// Error message if any
    pub error: Option<String>,
}

/// Export encrypted backup of deploy accounts
/// The backup is encrypted with the user's password for portability
#[tauri::command]
pub async fn export_deploy_backup(
    app: AppHandle,
    password: String,
) -> Result<BackupExportResult, String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }

    // Get all accounts (with decrypted tokens)
    let accounts = get_accounts_from_store(&app)?;
    let account_count = accounts.len();

    // Serialize accounts to JSON
    let accounts_json = serde_json::to_string(&accounts)
        .map_err(|e| format!("Failed to serialize accounts: {}", e))?;

    // Encrypt the accounts data with user's password
    let password_key = derive_password_key(&password);
    let cipher = aes_gcm::Aes256Gcm::new_from_slice(&password_key)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;

    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::Nonce;
    use rand::RngCore;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, accounts_json.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    let backup_data = EncryptedData {
        nonce: BASE64.encode(nonce_bytes),
        ciphertext: BASE64.encode(ciphertext),
    };

    Ok(BackupExportResult {
        encrypted_data: backup_data,
        account_count,
    })
}

/// Import encrypted backup of deploy accounts
/// The backup is decrypted with the user's password
#[tauri::command]
pub async fn import_deploy_backup(
    app: AppHandle,
    encrypted_data: EncryptedData,
    password: String,
) -> Result<BackupImportResult, String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::Nonce;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    // Derive key from password
    let password_key = derive_password_key(&password);
    let cipher = aes_gcm::Aes256Gcm::new_from_slice(&password_key)
        .map_err(|_| "Invalid encryption setup".to_string())?;

    // Decode and decrypt
    let nonce_bytes = BASE64
        .decode(&encrypted_data.nonce)
        .map_err(|_| "Invalid backup format".to_string())?;

    if nonce_bytes.len() != 12 {
        return Ok(BackupImportResult {
            success: false,
            accounts_restored: 0,
            error: Some("Invalid backup format".to_string()),
        });
    }

    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = BASE64
        .decode(&encrypted_data.ciphertext)
        .map_err(|_| "Invalid backup format".to_string())?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Wrong password or corrupted backup".to_string())?;

    let accounts_json =
        String::from_utf8(plaintext).map_err(|_| "Corrupted backup data".to_string())?;

    // Parse accounts
    let imported_accounts: Vec<DeployAccount> = serde_json::from_str(&accounts_json)
        .map_err(|e| format!("Failed to parse backup: {}", e))?;

    // Merge with existing accounts (avoid duplicates based on platform + platform_user_id)
    let repo = get_deploy_repo(&app);
    let existing = get_accounts_from_store(&app)?;
    let mut accounts_restored = 0;

    for imported in imported_accounts {
        let is_duplicate = existing.iter().any(|a| {
            a.platform == imported.platform && a.platform_user_id == imported.platform_user_id
        });

        if !is_duplicate {
            repo.save_account(&imported)?;
            accounts_restored += 1;
        }
    }

    Ok(BackupImportResult {
        success: true,
        accounts_restored,
        error: None,
    })
}

/// Derive encryption key from password (same algorithm as crypto.rs)
fn derive_password_key(password: &str) -> [u8; 32] {
    use sha2::Digest;

    let mut hasher = sha2::Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(b"specforge-backup-salt-v1");

    let mut result = hasher.finalize();

    // Additional rounds for basic stretching
    for _ in 0..10000 {
        let mut hasher = sha2::Sha256::new();
        hasher.update(&result);
        hasher.update(password.as_bytes());
        result = hasher.finalize();
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

// ============================================================================
// Deploy UI Enhancement Commands (018-deploy-ui-enhancement)
// ============================================================================

use crate::models::deploy::{
    CloudflareProjectInfo, DeploymentStats, LastSuccessfulDeployment,
    NetlifySiteInfo, PlatformSiteInfo,
};

/// Get deployment statistics for a project
/// Calculates stats from deployment history
#[tauri::command]
pub async fn get_deployment_stats(
    app: AppHandle,
    project_id: String,
) -> Result<DeploymentStats, String> {
    let deployments = get_deployments_from_store(&app, &project_id)?;

    let total = deployments.len();
    let successful: Vec<_> = deployments
        .iter()
        .filter(|d| d.status == DeploymentStatus::Ready)
        .collect();
    let failed = deployments
        .iter()
        .filter(|d| d.status == DeploymentStatus::Failed)
        .count();

    let success_rate = if total > 0 {
        (successful.len() as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    // Calculate deploy times from successful deployments
    let deploy_times: Vec<u64> = successful.iter().filter_map(|d| d.deploy_time).collect();

    let average_deploy_time = if !deploy_times.is_empty() {
        Some(deploy_times.iter().sum::<u64>() as f64 / deploy_times.len() as f64)
    } else {
        None
    };

    let fastest_deploy_time = deploy_times.iter().min().copied();
    let slowest_deploy_time = deploy_times.iter().max().copied();

    // Get last successful deployment
    let last_successful_deployment =
        successful
            .iter()
            .max_by_key(|d| d.completed_at)
            .and_then(|d| {
                d.url.as_ref().map(|url| LastSuccessfulDeployment {
                    id: d.id.clone(),
                    url: url.clone(),
                    deployed_at: d.completed_at.unwrap_or(d.created_at),
                    commit_hash: d.commit_hash.clone(),
                    platform: d.platform.clone(),
                })
            });

    // Count recent deployments (last 7 days)
    let seven_days_ago = chrono::Utc::now() - chrono::Duration::days(7);
    let recent_count = deployments
        .iter()
        .filter(|d| d.created_at > seven_days_ago)
        .count();

    Ok(DeploymentStats {
        total_deployments: total,
        successful_deployments: successful.len(),
        failed_deployments: failed,
        success_rate,
        average_deploy_time,
        fastest_deploy_time,
        slowest_deploy_time,
        last_successful_deployment,
        recent_deployments_count: recent_count,
    })
}

/// Get platform-specific site information
/// Fetches extended info from platform APIs
#[tauri::command]
pub async fn get_platform_site_info(
    app: AppHandle,
    project_id: String,
) -> Result<Option<PlatformSiteInfo>, String> {
    // Get deployment config for the project
    let config = match get_config_from_store(&app, &project_id)? {
        Some(c) => c,
        None => return Ok(None),
    };

    // Get access token from bound account
    let access_token = match get_deployment_access_token(&app, &config) {
        Ok(token) => token,
        Err(_) => return Ok(None),
    };

    match config.platform {
        PlatformType::Netlify => {
            let site_id = match &config.netlify_site_id {
                Some(id) => id,
                None => return Ok(None),
            };

            match fetch_netlify_site_info(&access_token, site_id).await {
                Ok(info) => Ok(Some(PlatformSiteInfo::Netlify { info })),
                Err(e) => {
                    log::warn!("Failed to fetch Netlify site info: {}", e);
                    Ok(None)
                }
            }
        }
        PlatformType::CloudflarePages => {
            let project_name = match &config.cloudflare_project_name {
                Some(name) => name.clone(),
                None => return Ok(None),
            };

            // Get account from bound account using repository
            let account = match config.account_id.as_ref() {
                Some(id) => find_account_by_id_from_store(&app, id)?,
                None => None,
            };

            let cf_account_id = match account {
                Some(acc) => acc.platform_user_id.clone(),
                None => return Ok(None),
            };

            match fetch_cloudflare_project_info(&access_token, &cf_account_id, &project_name).await
            {
                Ok(info) => Ok(Some(PlatformSiteInfo::CloudflarePages { info })),
                Err(e) => {
                    log::warn!("Failed to fetch Cloudflare project info: {}", e);
                    Ok(None)
                }
            }
        }
        PlatformType::GithubPages => {
            // For GitHub Pages, we need the project path to get repo info
            // This is a simplified version - ideally we'd get the project path from somewhere
            Ok(None)
        }
    }
}

/// Fetch extended site info from Netlify API
async fn fetch_netlify_site_info(
    access_token: &str,
    site_id: &str,
) -> Result<NetlifySiteInfo, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/{}", NETLIFY_SITES_URL, site_id);

    let response = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Netlify site: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Netlify API error: {}", response.status()));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Netlify response: {}", e))?;

    // Parse published_at
    let published_at = data["published_deploy"]["published_at"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    Ok(NetlifySiteInfo {
        site_id: data["id"].as_str().unwrap_or("").to_string(),
        name: data["name"].as_str().unwrap_or("").to_string(),
        url: data["url"].as_str().unwrap_or("").to_string(),
        ssl_url: data["ssl_url"].as_str().unwrap_or("").to_string(),
        screenshot_url: data["screenshot_url"].as_str().map(|s| s.to_string()),
        custom_domain: data["custom_domain"].as_str().map(|s| s.to_string()),
        ssl: data["ssl"].as_bool().unwrap_or(false),
        published_at,
        repo_url: data["build_settings"]["repo_url"]
            .as_str()
            .map(|s| s.to_string()),
        repo_branch: data["build_settings"]["repo_branch"]
            .as_str()
            .map(|s| s.to_string()),
        build_minutes_used: None, // Would need separate API call to /accounts/{account_id}/builds/status
        build_minutes_included: None,
        form_count: data["published_deploy"]["form_count"]
            .as_u64()
            .map(|n| n as usize),
        account_slug: data["account_slug"].as_str().map(|s| s.to_string()),
        account_name: data["account_name"].as_str().map(|s| s.to_string()),
    })
}

/// Fetch extended project info from Cloudflare Pages API
async fn fetch_cloudflare_project_info(
    access_token: &str,
    account_id: &str,
    project_name: &str,
) -> Result<CloudflareProjectInfo, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/accounts/{}/pages/projects/{}",
        CLOUDFLARE_API_BASE, account_id, project_name
    );

    let response = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Cloudflare project: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Cloudflare API error: {}", response.status()));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Cloudflare response: {}", e))?;

    let result = &data["result"];

    // Parse created_on
    let created_at = result["created_on"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);

    // Get domains
    let domains: Vec<String> = result["domains"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Get latest deployment info
    let latest_deployment = &result["latest_deployment"];
    let latest_deployment_url = latest_deployment["url"].as_str().map(|s| s.to_string());
    let latest_deployment_status = latest_deployment["latest_stage"]["status"]
        .as_str()
        .map(|s| s.to_string());

    Ok(CloudflareProjectInfo {
        name: result["name"].as_str().unwrap_or("").to_string(),
        subdomain: result["subdomain"].as_str().unwrap_or("").to_string(),
        domains,
        production_branch: result["production_branch"]
            .as_str()
            .unwrap_or("main")
            .to_string(),
        latest_deployment_url,
        latest_deployment_status,
        created_at,
        deployments_count: None, // Would need to count from deployments list
    })
}
