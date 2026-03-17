// Deploy Repository
// Handles all database operations for deploy accounts, configurations, preferences, and deployments

use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::models::deploy::{
    DeployAccount, Deployment, DeploymentConfig, DeploymentEnvironment, DeploymentStatus,
    DeployPreferences, EnvVariable, PlatformType,
};
use crate::utils::database::Database;

/// Maximum history entries per project
const MAX_HISTORY_PER_PROJECT: usize = 50;

/// Repository for deploy data access
pub struct DeployRepository {
    db: Database,
}

impl DeployRepository {
    /// Create a new DeployRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // =========================================================================
    // Deploy Accounts
    // =========================================================================

    /// List all deploy accounts
    pub fn list_accounts(&self) -> Result<Vec<DeployAccount>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, platform, platform_user_id, username, display_name,
                           avatar_url, access_token, connected_at, expires_at
                    FROM deploy_accounts
                    ORDER BY connected_at DESC
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(AccountRow {
                        id: row.get(0)?,
                        platform: row.get(1)?,
                        platform_user_id: row.get(2)?,
                        username: row.get(3)?,
                        display_name: row.get(4)?,
                        avatar_url: row.get(5)?,
                        access_token: row.get(6)?,
                        connected_at: row.get(7)?,
                        expires_at: row.get(8)?,
                    })
                })
                .map_err(|e| format!("Failed to query deploy accounts: {}", e))?;

            let mut accounts = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                accounts.push(row.into_account()?);
            }

            Ok(accounts)
        })
    }

    /// List accounts by platform
    pub fn list_accounts_by_platform(
        &self,
        platform: PlatformType,
    ) -> Result<Vec<DeployAccount>, String> {
        let platform_str = platform_to_string(&platform);

        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, platform, platform_user_id, username, display_name,
                           avatar_url, access_token, connected_at, expires_at
                    FROM deploy_accounts
                    WHERE platform = ?1
                    ORDER BY connected_at DESC
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![platform_str], |row| {
                    Ok(AccountRow {
                        id: row.get(0)?,
                        platform: row.get(1)?,
                        platform_user_id: row.get(2)?,
                        username: row.get(3)?,
                        display_name: row.get(4)?,
                        avatar_url: row.get(5)?,
                        access_token: row.get(6)?,
                        connected_at: row.get(7)?,
                        expires_at: row.get(8)?,
                    })
                })
                .map_err(|e| format!("Failed to query deploy accounts: {}", e))?;

            let mut accounts = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                accounts.push(row.into_account()?);
            }

            Ok(accounts)
        })
    }

    /// Get a deploy account by ID
    pub fn get_account(&self, id: &str) -> Result<Option<DeployAccount>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, platform, platform_user_id, username, display_name,
                       avatar_url, access_token, connected_at, expires_at
                FROM deploy_accounts
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(AccountRow {
                        id: row.get(0)?,
                        platform: row.get(1)?,
                        platform_user_id: row.get(2)?,
                        username: row.get(3)?,
                        display_name: row.get(4)?,
                        avatar_url: row.get(5)?,
                        access_token: row.get(6)?,
                        connected_at: row.get(7)?,
                        expires_at: row.get(8)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_account()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get deploy account: {}", e)),
            }
        })
    }

    /// Save a deploy account
    /// IMPORTANT: Uses ON CONFLICT DO UPDATE instead of INSERT OR REPLACE
    /// to avoid triggering ON DELETE CASCADE on deploy_account_tokens table.
    /// INSERT OR REPLACE internally does DELETE + INSERT which triggers cascades.
    pub fn save_account(&self, account: &DeployAccount) -> Result<(), String> {
        let platform_str = platform_to_string(&account.platform);
        let connected_at_str = account.connected_at.to_rfc3339();
        let expires_at_str = account.expires_at.map(|dt| dt.to_rfc3339());

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO deploy_accounts
                (id, platform, platform_user_id, username, display_name,
                 avatar_url, access_token, connected_at, expires_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(id) DO UPDATE SET
                    platform = excluded.platform,
                    platform_user_id = excluded.platform_user_id,
                    username = excluded.username,
                    display_name = excluded.display_name,
                    avatar_url = excluded.avatar_url,
                    access_token = excluded.access_token,
                    connected_at = excluded.connected_at,
                    expires_at = excluded.expires_at
                "#,
                params![
                    account.id,
                    platform_str,
                    account.platform_user_id,
                    account.username,
                    account.display_name,
                    account.avatar_url,
                    account.access_token,
                    connected_at_str,
                    expires_at_str,
                ],
            )
            .map_err(|e| format!("Failed to save deploy account: {}", e))?;

            Ok(())
        })
    }

    /// Delete a deploy account
    pub fn delete_account(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute("DELETE FROM deploy_accounts WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete deploy account: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    // =========================================================================
    // Deploy Account Tokens (Encrypted)
    // =========================================================================

    /// Store an encrypted access token for a deploy account
    pub fn store_token(
        &self,
        account_id: &str,
        ciphertext: &str,
        nonce: &str,
    ) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO deploy_account_tokens
                (account_id, ciphertext, nonce, updated_at)
                VALUES (?1, ?2, ?3, datetime('now'))
                "#,
                params![account_id, ciphertext, nonce],
            )
            .map_err(|e| format!("Failed to store deploy account token: {}", e))?;

            Ok(())
        })
    }

    /// Get encrypted token data for a deploy account
    /// Returns (ciphertext, nonce) if found
    pub fn get_token(&self, account_id: &str) -> Result<Option<(String, String)>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                "SELECT ciphertext, nonce FROM deploy_account_tokens WHERE account_id = ?1",
                params![account_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            );

            match result {
                Ok(data) => Ok(Some(data)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get deploy account token: {}", e)),
            }
        })
    }

    /// Delete encrypted token for a deploy account
    pub fn delete_token(&self, account_id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute(
                    "DELETE FROM deploy_account_tokens WHERE account_id = ?1",
                    params![account_id],
                )
                .map_err(|e| format!("Failed to delete deploy account token: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    /// Check if encrypted token exists for a deploy account
    pub fn has_token(&self, account_id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM deploy_account_tokens WHERE account_id = ?1",
                    params![account_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to check token: {}", e))?;

            Ok(count > 0)
        })
    }

    /// Get legacy plaintext token from deploy_accounts table (for migration)
    pub fn get_legacy_token(&self, account_id: &str) -> Result<Option<String>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                "SELECT access_token FROM deploy_accounts WHERE id = ?1",
                params![account_id],
                |row| row.get::<_, String>(0),
            );

            match result {
                Ok(token) if !token.is_empty() => Ok(Some(token)),
                Ok(_) => Ok(None),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get legacy token: {}", e)),
            }
        })
    }

    /// Clear legacy plaintext token from deploy_accounts table (after migration)
    pub fn clear_legacy_token(&self, account_id: &str) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE deploy_accounts SET access_token = '' WHERE id = ?1",
                params![account_id],
            )
            .map_err(|e| format!("Failed to clear legacy token: {}", e))?;

            Ok(())
        })
    }

    // =========================================================================
    // Deployment Configurations
    // =========================================================================

    /// Get deployment config for a project
    pub fn get_config(&self, project_id: &str) -> Result<Option<DeploymentConfig>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT project_id, platform, account_id, environment, framework_preset,
                       env_variables, root_directory, install_command, build_command,
                       output_directory, netlify_site_id, netlify_site_name,
                       cloudflare_account_id, cloudflare_project_name
                FROM deployment_configs
                WHERE project_id = ?1
                "#,
                params![project_id],
                |row| {
                    Ok(ConfigRow {
                        project_id: row.get(0)?,
                        platform: row.get(1)?,
                        account_id: row.get(2)?,
                        environment: row.get(3)?,
                        framework_preset: row.get(4)?,
                        env_variables: row.get(5)?,
                        root_directory: row.get(6)?,
                        install_command: row.get(7)?,
                        build_command: row.get(8)?,
                        output_directory: row.get(9)?,
                        netlify_site_id: row.get(10)?,
                        netlify_site_name: row.get(11)?,
                        cloudflare_account_id: row.get(12)?,
                        cloudflare_project_name: row.get(13)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_config()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get deployment config: {}", e)),
            }
        })
    }

    /// Save deployment config for a project
    pub fn save_config(&self, config: &DeploymentConfig) -> Result<(), String> {
        let platform_str = platform_to_string(&config.platform);
        let environment_str = environment_to_string(&config.environment);

        let env_vars_json = serde_json::to_string(&config.env_variables)
            .map_err(|e| format!("Failed to serialize env_variables: {}", e))?;

        log::debug!(
            "deploy_repo.save_config: project_id={}, platform={}, account_id={:?}, env_vars_count={}",
            config.project_id,
            platform_str,
            config.account_id,
            config.env_variables.len()
        );

        self.db.with_connection(|conn| {
            let rows_affected = conn.execute(
                r#"
                INSERT OR REPLACE INTO deployment_configs
                (project_id, platform, account_id, environment, framework_preset,
                 env_variables, root_directory, install_command, build_command,
                 output_directory, netlify_site_id, netlify_site_name,
                 cloudflare_account_id, cloudflare_project_name)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                "#,
                params![
                    config.project_id,
                    platform_str,
                    config.account_id,
                    environment_str,
                    config.framework_preset,
                    env_vars_json,
                    config.root_directory,
                    config.install_command,
                    config.build_command,
                    config.output_directory,
                    config.netlify_site_id,
                    config.netlify_site_name,
                    config.cloudflare_account_id,
                    config.cloudflare_project_name,
                ],
            )
            .map_err(|e| {
                log::error!("deploy_repo.save_config SQL error: {}", e);
                format!("Failed to save deployment config: {}", e)
            })?;

            log::debug!("deploy_repo.save_config: rows_affected={}", rows_affected);
            Ok(())
        })
    }

    /// Delete deployment config for a project
    pub fn delete_config(&self, project_id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute(
                    "DELETE FROM deployment_configs WHERE project_id = ?1",
                    params![project_id],
                )
                .map_err(|e| format!("Failed to delete deployment config: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    /// List all deployment configs
    pub fn list_configs(&self) -> Result<Vec<DeploymentConfig>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT project_id, platform, account_id, environment, framework_preset,
                           env_variables, root_directory, install_command, build_command,
                           output_directory, netlify_site_id, netlify_site_name,
                           cloudflare_account_id, cloudflare_project_name
                    FROM deployment_configs
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(ConfigRow {
                        project_id: row.get(0)?,
                        platform: row.get(1)?,
                        account_id: row.get(2)?,
                        environment: row.get(3)?,
                        framework_preset: row.get(4)?,
                        env_variables: row.get(5)?,
                        root_directory: row.get(6)?,
                        install_command: row.get(7)?,
                        build_command: row.get(8)?,
                        output_directory: row.get(9)?,
                        netlify_site_id: row.get(10)?,
                        netlify_site_name: row.get(11)?,
                        cloudflare_account_id: row.get(12)?,
                        cloudflare_project_name: row.get(13)?,
                    })
                })
                .map_err(|e| format!("Failed to query deployment configs: {}", e))?;

            let mut configs = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                configs.push(row.into_config()?);
            }

            Ok(configs)
        })
    }

    /// Find project IDs using a specific account
    pub fn find_projects_using_account(&self, account_id: &str) -> Result<Vec<String>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare("SELECT project_id FROM deployment_configs WHERE account_id = ?1")
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![account_id], |row| row.get(0))
                .map_err(|e| format!("Failed to query configs: {}", e))?;

            let mut project_ids = Vec::new();
            for row in rows {
                project_ids.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
            }

            Ok(project_ids)
        })
    }

    /// Clear account_id from all configs using a specific account
    pub fn clear_account_from_configs(&self, account_id: &str) -> Result<usize, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute(
                    "UPDATE deployment_configs SET account_id = NULL WHERE account_id = ?1",
                    params![account_id],
                )
                .map_err(|e| format!("Failed to clear account from configs: {}", e))?;

            Ok(rows_affected)
        })
    }

    // =========================================================================
    // Deploy Preferences
    // =========================================================================

    /// Get deploy preferences
    pub fn get_preferences(&self) -> Result<DeployPreferences, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT default_github_pages_account_id, default_netlify_account_id,
                       default_cloudflare_pages_account_id
                FROM deploy_preferences
                WHERE id = 1
                "#,
                [],
                |row| {
                    Ok(DeployPreferences {
                        default_github_pages_account_id: row.get(0)?,
                        default_netlify_account_id: row.get(1)?,
                        default_cloudflare_pages_account_id: row.get(2)?,
                    })
                },
            );

            match result {
                Ok(prefs) => Ok(prefs),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(DeployPreferences::default()),
                Err(e) => Err(format!("Failed to get deploy preferences: {}", e)),
            }
        })
    }

    /// Save deploy preferences
    pub fn save_preferences(&self, prefs: &DeployPreferences) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO deploy_preferences
                (id, default_github_pages_account_id, default_netlify_account_id,
                 default_cloudflare_pages_account_id)
                VALUES (1, ?1, ?2, ?3)
                "#,
                params![
                    prefs.default_github_pages_account_id,
                    prefs.default_netlify_account_id,
                    prefs.default_cloudflare_pages_account_id,
                ],
            )
            .map_err(|e| format!("Failed to save deploy preferences: {}", e))?;

            Ok(())
        })
    }

    // =========================================================================
    // Deployments (History)
    // =========================================================================

    /// List deployments for a project
    pub fn list_deployments(&self, project_id: &str) -> Result<Vec<Deployment>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, project_id, platform, status, url, created_at, completed_at,
                           commit_hash, commit_message, error_message, admin_url, deploy_time,
                           branch, site_name, preview_url
                    FROM deployments
                    WHERE project_id = ?1
                    ORDER BY created_at DESC
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![project_id], |row| {
                    Ok(DeploymentRow {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        platform: row.get(2)?,
                        status: row.get(3)?,
                        url: row.get(4)?,
                        created_at: row.get(5)?,
                        completed_at: row.get(6)?,
                        commit_hash: row.get(7)?,
                        commit_message: row.get(8)?,
                        error_message: row.get(9)?,
                        admin_url: row.get(10)?,
                        deploy_time: row.get(11)?,
                        branch: row.get(12)?,
                        site_name: row.get(13)?,
                        preview_url: row.get(14)?,
                    })
                })
                .map_err(|e| format!("Failed to query deployments: {}", e))?;

            let mut deployments = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                deployments.push(row.into_deployment()?);
            }

            Ok(deployments)
        })
    }

    /// Get a single deployment by ID
    pub fn get_deployment(&self, id: &str) -> Result<Option<Deployment>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, project_id, platform, status, url, created_at, completed_at,
                       commit_hash, commit_message, error_message, admin_url, deploy_time,
                       branch, site_name, preview_url
                FROM deployments
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(DeploymentRow {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        platform: row.get(2)?,
                        status: row.get(3)?,
                        url: row.get(4)?,
                        created_at: row.get(5)?,
                        completed_at: row.get(6)?,
                        commit_hash: row.get(7)?,
                        commit_message: row.get(8)?,
                        error_message: row.get(9)?,
                        admin_url: row.get(10)?,
                        deploy_time: row.get(11)?,
                        branch: row.get(12)?,
                        site_name: row.get(13)?,
                        preview_url: row.get(14)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_deployment()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get deployment: {}", e)),
            }
        })
    }

    /// Save a deployment (insert or update)
    pub fn save_deployment(&self, deployment: &Deployment) -> Result<(), String> {
        let platform_str = platform_to_string(&deployment.platform);
        let status_str = status_to_string(&deployment.status);
        let created_at_str = deployment.created_at.to_rfc3339();
        let completed_at_str = deployment.completed_at.map(|dt| dt.to_rfc3339());

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO deployments
                (id, project_id, platform, status, url, created_at, completed_at,
                 commit_hash, commit_message, error_message, admin_url, deploy_time,
                 branch, site_name, preview_url)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                "#,
                params![
                    deployment.id,
                    deployment.project_id,
                    platform_str,
                    status_str,
                    deployment.url,
                    created_at_str,
                    completed_at_str,
                    deployment.commit_hash,
                    deployment.commit_message,
                    deployment.error_message,
                    deployment.admin_url,
                    deployment.deploy_time.map(|t| t as i64),
                    deployment.branch,
                    deployment.site_name,
                    deployment.preview_url,
                ],
            )
            .map_err(|e| format!("Failed to save deployment: {}", e))?;

            // Trim old deployments to keep only MAX_HISTORY_PER_PROJECT
            self.trim_deployment_history(conn, &deployment.project_id)?;

            Ok(())
        })
    }

    /// Trim deployment history to MAX_HISTORY_PER_PROJECT entries
    fn trim_deployment_history(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
    ) -> Result<(), String> {
        // Get count of deployments for this project
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM deployments WHERE project_id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count deployments: {}", e))?;

        if count as usize > MAX_HISTORY_PER_PROJECT {
            // Delete oldest entries exceeding the limit
            conn.execute(
                r#"
                DELETE FROM deployments
                WHERE project_id = ?1 AND id NOT IN (
                    SELECT id FROM deployments
                    WHERE project_id = ?1
                    ORDER BY created_at DESC
                    LIMIT ?2
                )
                "#,
                params![project_id, MAX_HISTORY_PER_PROJECT as i64],
            )
            .map_err(|e| format!("Failed to trim deployment history: {}", e))?;
        }

        Ok(())
    }

    /// Delete a single deployment
    pub fn delete_deployment(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute("DELETE FROM deployments WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete deployment: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    /// Clear all deployments for a project
    pub fn clear_deployments(&self, project_id: &str) -> Result<usize, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute(
                    "DELETE FROM deployments WHERE project_id = ?1",
                    params![project_id],
                )
                .map_err(|e| format!("Failed to clear deployments: {}", e))?;

            Ok(rows_affected)
        })
    }

    /// Check if an account exists and matches the given platform
    pub fn account_exists_for_platform(
        &self,
        account_id: &str,
        platform: &PlatformType,
    ) -> Result<bool, String> {
        let platform_str = platform_to_string(platform);

        self.db.with_connection(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM deploy_accounts WHERE id = ?1 AND platform = ?2",
                    params![account_id, platform_str],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to check account: {}", e))?;

            Ok(count > 0)
        })
    }

    /// Check if account with same platform and platform_user_id exists
    pub fn account_exists_by_platform_user(
        &self,
        platform: &PlatformType,
        platform_user_id: &str,
    ) -> Result<bool, String> {
        let platform_str = platform_to_string(platform);

        self.db.with_connection(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM deploy_accounts WHERE platform = ?1 AND platform_user_id = ?2",
                    params![platform_str, platform_user_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to check account: {}", e))?;

            Ok(count > 0)
        })
    }

    /// Count accounts by platform
    pub fn count_accounts_by_platform(&self, platform: &PlatformType) -> Result<usize, String> {
        let platform_str = platform_to_string(platform);

        self.db.with_connection(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM deploy_accounts WHERE platform = ?1",
                    params![platform_str],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to count accounts: {}", e))?;

            Ok(count as usize)
        })
    }
}

/// Convert PlatformType to string
fn platform_to_string(platform: &PlatformType) -> &'static str {
    match platform {
        PlatformType::GithubPages => "github_pages",
        PlatformType::Netlify => "netlify",
        PlatformType::CloudflarePages => "cloudflare_pages",
    }
}

/// Convert string to PlatformType
fn string_to_platform(s: &str) -> PlatformType {
    match s {
        "github_pages" => PlatformType::GithubPages,
        "netlify" => PlatformType::Netlify,
        "cloudflare_pages" => PlatformType::CloudflarePages,
        _ => PlatformType::GithubPages,
    }
}

/// Convert DeploymentEnvironment to string
fn environment_to_string(env: &DeploymentEnvironment) -> &'static str {
    match env {
        DeploymentEnvironment::Production => "production",
        DeploymentEnvironment::Preview => "preview",
    }
}

/// Convert string to DeploymentEnvironment
fn string_to_environment(s: &str) -> DeploymentEnvironment {
    match s.to_lowercase().as_str() {
        "preview" => DeploymentEnvironment::Preview,
        _ => DeploymentEnvironment::Production,
    }
}

/// Convert DeploymentStatus to string
fn status_to_string(status: &DeploymentStatus) -> &'static str {
    match status {
        DeploymentStatus::Queued => "queued",
        DeploymentStatus::Building => "building",
        DeploymentStatus::Deploying => "deploying",
        DeploymentStatus::Ready => "ready",
        DeploymentStatus::Failed => "failed",
        DeploymentStatus::Cancelled => "cancelled",
    }
}

/// Convert string to DeploymentStatus
fn string_to_status(s: &str) -> DeploymentStatus {
    match s.to_lowercase().as_str() {
        "queued" => DeploymentStatus::Queued,
        "building" => DeploymentStatus::Building,
        "deploying" => DeploymentStatus::Deploying,
        "ready" => DeploymentStatus::Ready,
        "failed" => DeploymentStatus::Failed,
        "cancelled" => DeploymentStatus::Cancelled,
        _ => DeploymentStatus::Queued,
    }
}

/// Internal row structure for deploy accounts
struct AccountRow {
    id: String,
    platform: String,
    platform_user_id: String,
    username: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
    access_token: String,
    connected_at: String,
    expires_at: Option<String>,
}

impl AccountRow {
    fn into_account(self) -> Result<DeployAccount, String> {
        let connected_at = DateTime::parse_from_rfc3339(&self.connected_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let expires_at = self
            .expires_at
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(DeployAccount {
            id: self.id,
            platform: string_to_platform(&self.platform),
            platform_user_id: self.platform_user_id,
            username: self.username,
            display_name: self.display_name,
            avatar_url: self.avatar_url,
            access_token: self.access_token,
            connected_at,
            expires_at,
        })
    }
}

/// Internal row structure for deployment configs
struct ConfigRow {
    project_id: String,
    platform: String,
    account_id: Option<String>,
    environment: Option<String>,
    framework_preset: Option<String>,
    env_variables: Option<String>,
    root_directory: Option<String>,
    install_command: Option<String>,
    build_command: Option<String>,
    output_directory: Option<String>,
    netlify_site_id: Option<String>,
    netlify_site_name: Option<String>,
    cloudflare_account_id: Option<String>,
    cloudflare_project_name: Option<String>,
}

impl ConfigRow {
    fn into_config(self) -> Result<DeploymentConfig, String> {
        let env_variables: Vec<EnvVariable> = self
            .env_variables
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default();

        let environment = self
            .environment
            .as_ref()
            .map(|s| string_to_environment(s))
            .unwrap_or_default();

        Ok(DeploymentConfig {
            project_id: self.project_id,
            platform: string_to_platform(&self.platform),
            account_id: self.account_id,
            environment,
            framework_preset: self.framework_preset,
            env_variables,
            root_directory: self.root_directory,
            install_command: self.install_command,
            build_command: self.build_command,
            output_directory: self.output_directory,
            netlify_site_id: self.netlify_site_id,
            netlify_site_name: self.netlify_site_name,
            cloudflare_account_id: self.cloudflare_account_id,
            cloudflare_project_name: self.cloudflare_project_name,
        })
    }
}

/// Internal row structure for deployments
struct DeploymentRow {
    id: String,
    project_id: String,
    platform: String,
    status: String,
    url: Option<String>,
    created_at: String,
    completed_at: Option<String>,
    commit_hash: Option<String>,
    commit_message: Option<String>,
    error_message: Option<String>,
    admin_url: Option<String>,
    deploy_time: Option<i64>,
    branch: Option<String>,
    site_name: Option<String>,
    preview_url: Option<String>,
}

impl DeploymentRow {
    fn into_deployment(self) -> Result<Deployment, String> {
        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let completed_at = self
            .completed_at
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(Deployment {
            id: self.id,
            project_id: self.project_id,
            platform: string_to_platform(&self.platform),
            status: string_to_status(&self.status),
            url: self.url,
            created_at,
            completed_at,
            commit_hash: self.commit_hash,
            commit_message: self.commit_message,
            error_message: self.error_message,
            admin_url: self.admin_url,
            deploy_time: self.deploy_time.map(|t| t as u64),
            branch: self.branch,
            site_name: self.site_name,
            preview_url: self.preview_url,
        })
    }
}
