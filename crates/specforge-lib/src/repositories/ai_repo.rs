// AI Repository
// Handles all database operations for AI providers and templates

use chrono::Utc;
use rusqlite::params;

use crate::models::ai::{AIProvider, AIProviderConfig, CommitFormat, PromptTemplate, TemplateCategory};
use crate::utils::database::Database;

/// Repository for AI provider and template data access
pub struct AIRepository {
    db: Database,
}

impl AIRepository {
    /// Create a new AIRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // =========================================================================
    // AI Providers
    // =========================================================================

    /// List all AI providers
    pub fn list_providers(&self) -> Result<Vec<AIProviderConfig>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, name, provider, endpoint, model, is_default, is_enabled,
                           created_at, updated_at
                    FROM ai_providers
                    ORDER BY name
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(AIProviderRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        provider: row.get(2)?,
                        endpoint: row.get(3)?,
                        model: row.get(4)?,
                        is_default: row.get(5)?,
                        is_enabled: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                })
                .map_err(|e| format!("Failed to query AI providers: {}", e))?;

            let mut providers = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                providers.push(row.into_provider()?);
            }

            Ok(providers)
        })
    }

    /// Get an AI provider by ID
    pub fn get_provider(&self, id: &str) -> Result<Option<AIProviderConfig>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, name, provider, endpoint, model, is_default, is_enabled,
                       created_at, updated_at
                FROM ai_providers
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(AIProviderRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        provider: row.get(2)?,
                        endpoint: row.get(3)?,
                        model: row.get(4)?,
                        is_default: row.get(5)?,
                        is_enabled: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_provider()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get AI provider: {}", e)),
            }
        })
    }

    /// Get the default AI provider
    pub fn get_default_provider(&self) -> Result<Option<AIProviderConfig>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, name, provider, endpoint, model, is_default, is_enabled,
                       created_at, updated_at
                FROM ai_providers
                WHERE is_default = 1 AND is_enabled = 1
                LIMIT 1
                "#,
                [],
                |row| {
                    Ok(AIProviderRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        provider: row.get(2)?,
                        endpoint: row.get(3)?,
                        model: row.get(4)?,
                        is_default: row.get(5)?,
                        is_enabled: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_provider()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get default AI provider: {}", e)),
            }
        })
    }

    /// Save an AI provider
    pub fn save_provider(&self, provider: &AIProviderConfig) -> Result<(), String> {
        let provider_str = provider.provider.to_string();
        let now = Utc::now().to_rfc3339();

        self.db.with_connection(|conn| {
            // Use INSERT ... ON CONFLICT DO UPDATE to avoid triggering ON DELETE CASCADE
            // INSERT OR REPLACE would delete the old row first, which cascades to ai_api_keys
            conn.execute(
                r#"
                INSERT INTO ai_providers
                (id, name, provider, endpoint, model, is_default, is_enabled, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    provider = excluded.provider,
                    endpoint = excluded.endpoint,
                    model = excluded.model,
                    is_default = excluded.is_default,
                    is_enabled = excluded.is_enabled,
                    updated_at = excluded.updated_at
                "#,
                params![
                    provider.id,
                    provider.name,
                    provider_str,
                    provider.endpoint,
                    provider.model,
                    provider.is_default as i32,
                    provider.is_enabled as i32,
                    provider.created_at.to_rfc3339(),
                    now,
                ],
            )
            .map_err(|e| format!("Failed to save AI provider: {}", e))?;

            Ok(())
        })
    }

    /// Delete an AI provider
    pub fn delete_provider(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute("DELETE FROM ai_providers WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete AI provider: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    // =========================================================================
    // AI Templates
    // =========================================================================

    /// List all AI templates
    pub fn list_templates(&self) -> Result<Vec<PromptTemplate>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, name, description, category, template, output_format,
                           is_default, is_builtin, created_at, updated_at
                    FROM ai_templates
                    ORDER BY name
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(TemplateRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        category: row.get(3)?,
                        template: row.get(4)?,
                        output_format: row.get(5)?,
                        is_default: row.get(6)?,
                        is_builtin: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                })
                .map_err(|e| format!("Failed to query AI templates: {}", e))?;

            let mut templates = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                templates.push(row.into_template()?);
            }

            Ok(templates)
        })
    }

    /// List templates by category
    pub fn list_templates_by_category(
        &self,
        category: TemplateCategory,
    ) -> Result<Vec<PromptTemplate>, String> {
        let category_str = template_category_to_string(&category);

        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, name, description, category, template, output_format,
                           is_default, is_builtin, created_at, updated_at
                    FROM ai_templates
                    WHERE category = ?1
                    ORDER BY name
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![category_str], |row| {
                    Ok(TemplateRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        category: row.get(3)?,
                        template: row.get(4)?,
                        output_format: row.get(5)?,
                        is_default: row.get(6)?,
                        is_builtin: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                })
                .map_err(|e| format!("Failed to query AI templates: {}", e))?;

            let mut templates = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                templates.push(row.into_template()?);
            }

            Ok(templates)
        })
    }

    /// Save an AI template
    pub fn save_template(&self, template: &PromptTemplate) -> Result<(), String> {
        let category_str = template_category_to_string(&template.category);
        let now = Utc::now().to_rfc3339();
        let output_format_str = template.output_format.as_ref().map(commit_format_to_string);

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO ai_templates
                (id, name, description, category, template, output_format,
                 is_default, is_builtin, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    template.id,
                    template.name,
                    template.description,
                    category_str,
                    template.template,
                    output_format_str,
                    template.is_default as i32,
                    template.is_builtin as i32,
                    template.created_at.to_rfc3339(),
                    now,
                ],
            )
            .map_err(|e| format!("Failed to save AI template: {}", e))?;

            Ok(())
        })
    }

    /// Delete an AI template
    pub fn delete_template(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute("DELETE FROM ai_templates WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete AI template: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    // =========================================================================
    // Project AI Settings
    // =========================================================================

    /// Get project-specific AI settings
    pub fn get_project_settings(
        &self,
        project_path: &str,
    ) -> Result<crate::models::ai::ProjectAISettings, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT project_path, preferred_provider_id, preferred_template_id
                FROM project_ai_settings
                WHERE project_path = ?1
                "#,
                params![project_path],
                |row| {
                    Ok(crate::models::ai::ProjectAISettings {
                        project_path: row.get(0)?,
                        preferred_provider_id: row.get(1)?,
                        preferred_template_id: row.get(2)?,
                    })
                },
            );

            match result {
                Ok(settings) => Ok(settings),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    Ok(crate::models::ai::ProjectAISettings {
                        project_path: project_path.to_string(),
                        preferred_provider_id: None,
                        preferred_template_id: None,
                    })
                }
                Err(e) => Err(format!("Failed to get project AI settings: {}", e)),
            }
        })
    }

    /// Save project-specific AI settings
    pub fn save_project_settings(
        &self,
        settings: &crate::models::ai::ProjectAISettings,
    ) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO project_ai_settings
                (project_path, preferred_provider_id, preferred_template_id)
                VALUES (?1, ?2, ?3)
                "#,
                params![
                    settings.project_path,
                    settings.preferred_provider_id,
                    settings.preferred_template_id,
                ],
            )
            .map_err(|e| format!("Failed to save project AI settings: {}", e))?;

            Ok(())
        })
    }

    /// Set default provider (clears other defaults first)
    pub fn set_default_provider(&self, id: &str) -> Result<(), String> {
        self.db.with_connection(|conn| {
            // Clear all defaults
            conn.execute("UPDATE ai_providers SET is_default = 0", [])
                .map_err(|e| format!("Failed to clear default providers: {}", e))?;

            // Set new default
            let rows = conn
                .execute(
                    "UPDATE ai_providers SET is_default = 1 WHERE id = ?1",
                    params![id],
                )
                .map_err(|e| format!("Failed to set default provider: {}", e))?;

            if rows == 0 {
                return Err(format!("Provider not found: {}", id));
            }

            Ok(())
        })
    }

    /// Set default template for a category (clears other defaults in same category first)
    pub fn set_default_template(&self, id: &str) -> Result<(), String> {
        self.db.with_connection(|conn| {
            // Get the category of the target template
            let category: String = conn
                .query_row(
                    "SELECT category FROM ai_templates WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Template not found: {}", e))?;

            // Clear defaults only within the same category
            conn.execute(
                "UPDATE ai_templates SET is_default = 0 WHERE category = ?1",
                params![category],
            )
            .map_err(|e| format!("Failed to clear default templates: {}", e))?;

            // Set new default
            conn.execute(
                "UPDATE ai_templates SET is_default = 1 WHERE id = ?1",
                params![id],
            )
            .map_err(|e| format!("Failed to set default template: {}", e))?;

            Ok(())
        })
    }

    /// Get template by ID
    pub fn get_template(&self, id: &str) -> Result<Option<PromptTemplate>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, name, description, category, template, output_format,
                       is_default, is_builtin, created_at, updated_at
                FROM ai_templates
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(TemplateRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        category: row.get(3)?,
                        template: row.get(4)?,
                        output_format: row.get(5)?,
                        is_default: row.get(6)?,
                        is_builtin: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_template()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get template: {}", e)),
            }
        })
    }

    /// Get default template for a specific category
    pub fn get_default_template(
        &self,
        category: Option<&TemplateCategory>,
    ) -> Result<Option<PromptTemplate>, String> {
        self.db.with_connection(|conn| {
            let result = if let Some(cat) = category {
                let category_str = template_category_to_string(cat);
                conn.query_row(
                    r#"
                    SELECT id, name, description, category, template, output_format,
                           is_default, is_builtin, created_at, updated_at
                    FROM ai_templates
                    WHERE is_default = 1 AND category = ?1
                    LIMIT 1
                    "#,
                    params![category_str],
                    |row| {
                        Ok(TemplateRow {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            description: row.get(2)?,
                            category: row.get(3)?,
                            template: row.get(4)?,
                            output_format: row.get(5)?,
                            is_default: row.get(6)?,
                            is_builtin: row.get(7)?,
                            created_at: row.get(8)?,
                            updated_at: row.get(9)?,
                        })
                    },
                )
            } else {
                conn.query_row(
                    r#"
                    SELECT id, name, description, category, template, output_format,
                           is_default, is_builtin, created_at, updated_at
                    FROM ai_templates
                    WHERE is_default = 1
                    LIMIT 1
                    "#,
                    [],
                    |row| {
                        Ok(TemplateRow {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            description: row.get(2)?,
                            category: row.get(3)?,
                            template: row.get(4)?,
                            output_format: row.get(5)?,
                            is_default: row.get(6)?,
                            is_builtin: row.get(7)?,
                            created_at: row.get(8)?,
                            updated_at: row.get(9)?,
                        })
                    },
                )
            };

            match result {
                Ok(row) => Ok(Some(row.into_template()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get default template: {}", e)),
            }
        })
    }

    /// Check if a provider name already exists (excluding given ID)
    pub fn provider_name_exists(&self, name: &str, exclude_id: Option<&str>) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let count: i64 = if let Some(id) = exclude_id {
                conn.query_row(
                    "SELECT COUNT(*) FROM ai_providers WHERE name = ?1 AND id != ?2",
                    params![name, id],
                    |row| row.get(0),
                )
            } else {
                conn.query_row(
                    "SELECT COUNT(*) FROM ai_providers WHERE name = ?1",
                    params![name],
                    |row| row.get(0),
                )
            }
            .map_err(|e| format!("Failed to check provider name: {}", e))?;

            Ok(count > 0)
        })
    }

    /// Check if a template name already exists (excluding given ID)
    pub fn template_name_exists(&self, name: &str, exclude_id: Option<&str>) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let count: i64 = if let Some(id) = exclude_id {
                conn.query_row(
                    "SELECT COUNT(*) FROM ai_templates WHERE name = ?1 AND id != ?2",
                    params![name, id],
                    |row| row.get(0),
                )
            } else {
                conn.query_row(
                    "SELECT COUNT(*) FROM ai_templates WHERE name = ?1",
                    params![name],
                    |row| row.get(0),
                )
            }
            .map_err(|e| format!("Failed to check template name: {}", e))?;

            Ok(count > 0)
        })
    }

    // =========================================================================
    // API Key Management (encrypted storage)
    // =========================================================================

    /// Store an encrypted API key for a provider
    pub fn store_api_key(
        &self,
        provider_id: &str,
        ciphertext: &str,
        nonce: &str,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO ai_api_keys
                (provider_id, ciphertext, nonce, created_at, updated_at)
                VALUES (?1, ?2, ?3, COALESCE((SELECT created_at FROM ai_api_keys WHERE provider_id = ?1), ?4), ?4)
                "#,
                params![provider_id, ciphertext, nonce, now],
            )
            .map_err(|e| format!("Failed to store API key: {}", e))?;

            Ok(())
        })
    }

    /// Get encrypted API key for a provider
    pub fn get_api_key(&self, provider_id: &str) -> Result<Option<(String, String)>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                "SELECT ciphertext, nonce FROM ai_api_keys WHERE provider_id = ?1",
                params![provider_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            );

            match result {
                Ok((ciphertext, nonce)) => Ok(Some((ciphertext, nonce))),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get API key: {}", e)),
            }
        })
    }

    /// Delete API key for a provider
    pub fn delete_api_key(&self, provider_id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows = conn
                .execute("DELETE FROM ai_api_keys WHERE provider_id = ?1", params![provider_id])
                .map_err(|e| format!("Failed to delete API key: {}", e))?;

            Ok(rows > 0)
        })
    }

    /// Check if an API key exists for a provider
    pub fn has_api_key(&self, provider_id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM ai_api_keys WHERE provider_id = ?1",
                    params![provider_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to check API key: {}", e))?;

            Ok(count > 0)
        })
    }

    /// List all provider IDs that have stored API keys
    pub fn list_provider_ids_with_keys(&self) -> Result<Vec<String>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare("SELECT provider_id FROM ai_api_keys")
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| row.get(0))
                .map_err(|e| format!("Failed to list API keys: {}", e))?;

            let mut ids = Vec::new();
            for row in rows {
                ids.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
            }

            Ok(ids)
        })
    }

    // =========================================================================
    // CLI Tools (Feature 020: AI CLI Integration)
    // =========================================================================

    /// List all CLI tool configurations
    pub fn list_cli_tools(&self) -> Result<Vec<crate::models::cli_tool::CLIToolConfig>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, tool_type, name, binary_path, is_enabled, auth_mode,
                           api_key_provider_id, created_at, updated_at
                    FROM cli_tools
                    ORDER BY name
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(CLIToolRow {
                        id: row.get(0)?,
                        tool_type: row.get(1)?,
                        name: row.get(2)?,
                        binary_path: row.get(3)?,
                        is_enabled: row.get(4)?,
                        auth_mode: row.get(5)?,
                        api_key_provider_id: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                })
                .map_err(|e| format!("Failed to query CLI tools: {}", e))?;

            let mut tools = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                tools.push(row.into_config()?);
            }

            Ok(tools)
        })
    }

    /// Get CLI tool by ID
    pub fn get_cli_tool(&self, id: &str) -> Result<Option<crate::models::cli_tool::CLIToolConfig>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, tool_type, name, binary_path, is_enabled, auth_mode,
                       api_key_provider_id, created_at, updated_at
                FROM cli_tools
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(CLIToolRow {
                        id: row.get(0)?,
                        tool_type: row.get(1)?,
                        name: row.get(2)?,
                        binary_path: row.get(3)?,
                        is_enabled: row.get(4)?,
                        auth_mode: row.get(5)?,
                        api_key_provider_id: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_config()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get CLI tool: {}", e)),
            }
        })
    }

    /// Get CLI tool by type
    pub fn get_cli_tool_by_type(
        &self,
        tool_type: crate::models::cli_tool::CLIToolType,
    ) -> Result<Option<crate::models::cli_tool::CLIToolConfig>, String> {
        self.db.with_connection(|conn| {
            let type_str = cli_tool_type_to_string(&tool_type);
            let result = conn.query_row(
                r#"
                SELECT id, tool_type, name, binary_path, is_enabled, auth_mode,
                       api_key_provider_id, created_at, updated_at
                FROM cli_tools
                WHERE tool_type = ?1
                "#,
                params![type_str],
                |row| {
                    Ok(CLIToolRow {
                        id: row.get(0)?,
                        tool_type: row.get(1)?,
                        name: row.get(2)?,
                        binary_path: row.get(3)?,
                        is_enabled: row.get(4)?,
                        auth_mode: row.get(5)?,
                        api_key_provider_id: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_config()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get CLI tool: {}", e)),
            }
        })
    }

    /// Save CLI tool configuration (upsert)
    pub fn save_cli_tool(&self, config: &crate::models::cli_tool::CLIToolConfig) -> Result<(), String> {
        self.db.with_connection(|conn| {
            let type_str = cli_tool_type_to_string(&config.tool_type);
            let auth_mode_str = cli_auth_mode_to_string(&config.auth_mode);
            let now = Utc::now().to_rfc3339();

            conn.execute(
                r#"
                INSERT INTO cli_tools (id, tool_type, name, binary_path, is_enabled, auth_mode,
                                       api_key_provider_id, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(id) DO UPDATE SET
                    tool_type = excluded.tool_type,
                    name = excluded.name,
                    binary_path = excluded.binary_path,
                    is_enabled = excluded.is_enabled,
                    auth_mode = excluded.auth_mode,
                    api_key_provider_id = excluded.api_key_provider_id,
                    updated_at = excluded.updated_at
                "#,
                params![
                    config.id,
                    type_str,
                    config.name,
                    config.binary_path,
                    config.is_enabled as i32,
                    auth_mode_str,
                    config.api_key_provider_id,
                    config.created_at.to_rfc3339(),
                    now,
                ],
            )
            .map_err(|e| format!("Failed to save CLI tool: {}", e))?;

            Ok(())
        })
    }

    /// Delete CLI tool configuration
    pub fn delete_cli_tool(&self, id: &str) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute("DELETE FROM cli_tools WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete CLI tool: {}", e))?;
            Ok(())
        })
    }

    /// Log CLI execution (for audit)
    pub fn log_cli_execution(
        &self,
        tool_type: crate::models::cli_tool::CLIToolType,
        project_path: Option<&str>,
        prompt: &str,
        exit_code: Option<i32>,
        duration_ms: Option<u64>,
    ) -> Result<(), String> {
        use sha2::{Digest, Sha256};

        self.db.with_connection(|conn| {
            let type_str = cli_tool_type_to_string(&tool_type);
            let id = uuid::Uuid::new_v4().to_string();

            // Hash the prompt for privacy
            let mut hasher = Sha256::new();
            hasher.update(prompt.as_bytes());
            let prompt_hash = format!("{:x}", hasher.finalize());

            conn.execute(
                r#"
                INSERT INTO cli_execution_logs (id, tool_type, project_path, prompt_hash,
                                                execution_time_ms, exit_code, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
                "#,
                params![
                    id,
                    type_str,
                    project_path,
                    prompt_hash,
                    duration_ms.map(|d| d as i64),
                    exit_code,
                ],
            )
            .map_err(|e| format!("Failed to log CLI execution: {}", e))?;

            Ok(())
        })
    }

    /// Get CLI execution history
    pub fn get_cli_execution_history(
        &self,
        project_path: Option<&str>,
        limit: usize,
    ) -> Result<Vec<crate::models::cli_tool::CLIExecutionLog>, String> {
        self.db.with_connection(|conn| {
            let mut logs = Vec::new();

            if let Some(path) = project_path {
                let mut stmt = conn
                    .prepare(
                        r#"
                        SELECT id, tool_type, project_path, prompt_hash, model,
                               execution_time_ms, exit_code, tokens_used, created_at
                        FROM cli_execution_logs
                        WHERE project_path = ?1
                        ORDER BY created_at DESC
                        LIMIT ?2
                        "#,
                    )
                    .map_err(|e| format!("Failed to prepare statement: {}", e))?;

                let rows = stmt
                    .query_map(params![path, limit as i64], |row| {
                        Ok(CLILogRow {
                            id: row.get(0)?,
                            tool_type: row.get(1)?,
                            project_path: row.get(2)?,
                            prompt_hash: row.get(3)?,
                            model: row.get(4)?,
                            execution_time_ms: row.get(5)?,
                            exit_code: row.get(6)?,
                            tokens_used: row.get(7)?,
                            created_at: row.get(8)?,
                        })
                    })
                    .map_err(|e| format!("Failed to query CLI logs: {}", e))?;

                for row in rows {
                    let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                    logs.push(row.into_log()?);
                }
            } else {
                let mut stmt = conn
                    .prepare(
                        r#"
                        SELECT id, tool_type, project_path, prompt_hash, model,
                               execution_time_ms, exit_code, tokens_used, created_at
                        FROM cli_execution_logs
                        ORDER BY created_at DESC
                        LIMIT ?1
                        "#,
                    )
                    .map_err(|e| format!("Failed to prepare statement: {}", e))?;

                let rows = stmt
                    .query_map(params![limit as i64], |row| {
                        Ok(CLILogRow {
                            id: row.get(0)?,
                            tool_type: row.get(1)?,
                            project_path: row.get(2)?,
                            prompt_hash: row.get(3)?,
                            model: row.get(4)?,
                            execution_time_ms: row.get(5)?,
                            exit_code: row.get(6)?,
                            tokens_used: row.get(7)?,
                            created_at: row.get(8)?,
                        })
                    })
                    .map_err(|e| format!("Failed to query CLI logs: {}", e))?;

                for row in rows {
                    let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                    logs.push(row.into_log()?);
                }
            }

            Ok(logs)
        })
    }

    /// Clear CLI execution history
    pub fn clear_cli_execution_history(&self, project_path: Option<&str>) -> Result<(), String> {
        self.db.with_connection(|conn| {
            if let Some(path) = project_path {
                conn.execute(
                    "DELETE FROM cli_execution_logs WHERE project_path = ?1",
                    params![path],
                )
            } else {
                conn.execute("DELETE FROM cli_execution_logs", [])
            }
            .map_err(|e| format!("Failed to clear CLI logs: {}", e))?;
            Ok(())
        })
    }
}

/// Internal row structure for AI providers
struct AIProviderRow {
    id: String,
    name: String,
    provider: String,
    endpoint: String,
    model: String,
    is_default: i32,
    is_enabled: i32,
    created_at: String,
    updated_at: String,
}

impl AIProviderRow {
    fn into_provider(self) -> Result<AIProviderConfig, String> {
        use chrono::DateTime;

        let provider = match self.provider.as_str() {
            "openai" => AIProvider::OpenAI,
            "anthropic" => AIProvider::Anthropic,
            "gemini" => AIProvider::Gemini,
            "ollama" => AIProvider::Ollama,
            "lm_studio" => AIProvider::LMStudio,
            _ => AIProvider::OpenAI,
        };

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&self.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(AIProviderConfig {
            id: self.id,
            name: self.name,
            provider,
            endpoint: self.endpoint,
            model: self.model,
            is_default: self.is_default != 0,
            is_enabled: self.is_enabled != 0,
            created_at,
            updated_at,
        })
    }
}

/// Internal row structure for AI templates
struct TemplateRow {
    id: String,
    name: String,
    description: Option<String>,
    category: String,
    template: String,
    output_format: Option<String>,
    is_default: i32,
    is_builtin: i32,
    created_at: String,
    updated_at: String,
}

impl TemplateRow {
    fn into_template(self) -> Result<PromptTemplate, String> {
        use chrono::DateTime;

        let category = match self.category.as_str() {
            "git_commit" => TemplateCategory::GitCommit,
            "pull_request" => TemplateCategory::PullRequest,
            "code_review" => TemplateCategory::CodeReview,
            "documentation" => TemplateCategory::Documentation,
            "release_notes" => TemplateCategory::ReleaseNotes,
            "security_advisory" => TemplateCategory::SecurityAdvisory,
            "custom" => TemplateCategory::Custom,
            _ => TemplateCategory::Custom,
        };

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&self.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let output_format = self.output_format.as_ref().map(|s| string_to_commit_format(s));

        Ok(PromptTemplate {
            id: self.id,
            name: self.name,
            description: self.description,
            category,
            template: self.template,
            output_format,
            is_default: self.is_default != 0,
            is_builtin: self.is_builtin != 0,
            created_at,
            updated_at,
        })
    }
}

/// Convert CommitFormat to string
fn commit_format_to_string(format: &CommitFormat) -> String {
    match format {
        CommitFormat::ConventionalCommits => "conventional_commits".to_string(),
        CommitFormat::Simple => "simple".to_string(),
        CommitFormat::Custom => "custom".to_string(),
    }
}

/// Convert string to CommitFormat
fn string_to_commit_format(s: &str) -> CommitFormat {
    match s {
        "conventional_commits" => CommitFormat::ConventionalCommits,
        "simple" => CommitFormat::Simple,
        "custom" => CommitFormat::Custom,
        _ => CommitFormat::ConventionalCommits,
    }
}

/// Convert TemplateCategory to database string (snake_case)
fn template_category_to_string(category: &TemplateCategory) -> &'static str {
    match category {
        TemplateCategory::GitCommit => "git_commit",
        TemplateCategory::PullRequest => "pull_request",
        TemplateCategory::CodeReview => "code_review",
        TemplateCategory::Documentation => "documentation",
        TemplateCategory::ReleaseNotes => "release_notes",
        TemplateCategory::SecurityAdvisory => "security_advisory",
        TemplateCategory::Custom => "custom",
    }
}

// =========================================================================
// CLI Tool Helper Types and Functions (Feature 020)
// =========================================================================

/// Internal row structure for CLI tools
struct CLIToolRow {
    id: String,
    tool_type: String,
    name: String,
    binary_path: Option<String>,
    is_enabled: i32,
    auth_mode: String,
    api_key_provider_id: Option<String>,
    created_at: String,
    updated_at: String,
}

impl CLIToolRow {
    fn into_config(self) -> Result<crate::models::cli_tool::CLIToolConfig, String> {
        use crate::models::cli_tool::CLIToolConfig;
        use chrono::{DateTime, Utc};

        let tool_type = string_to_cli_tool_type(&self.tool_type)?;
        let auth_mode = string_to_cli_auth_mode(&self.auth_mode);

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&self.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(CLIToolConfig {
            id: self.id,
            tool_type,
            name: self.name,
            binary_path: self.binary_path,
            is_enabled: self.is_enabled != 0,
            auth_mode,
            api_key_provider_id: self.api_key_provider_id,
            created_at,
            updated_at,
        })
    }
}

/// Internal row structure for CLI execution logs
struct CLILogRow {
    id: String,
    tool_type: String,
    project_path: Option<String>,
    prompt_hash: String,
    model: Option<String>,
    execution_time_ms: Option<i64>,
    exit_code: Option<i32>,
    tokens_used: Option<i32>,
    created_at: String,
}

impl CLILogRow {
    fn into_log(self) -> Result<crate::models::cli_tool::CLIExecutionLog, String> {
        use crate::models::cli_tool::CLIExecutionLog;
        use chrono::{DateTime, Utc};

        let tool_type = string_to_cli_tool_type(&self.tool_type)?;

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(CLIExecutionLog {
            id: self.id,
            tool_type,
            project_path: self.project_path,
            prompt_hash: self.prompt_hash,
            model: self.model,
            execution_time_ms: self.execution_time_ms.map(|d| d as u64),
            exit_code: self.exit_code,
            tokens_used: self.tokens_used.map(|t| t as u32),
            created_at,
        })
    }
}

/// Convert CLIToolType to database string
fn cli_tool_type_to_string(tool_type: &crate::models::cli_tool::CLIToolType) -> &'static str {
    use crate::models::cli_tool::CLIToolType;
    match tool_type {
        CLIToolType::ClaudeCode => "claude_code",
        CLIToolType::Codex => "codex",
        CLIToolType::GeminiCli => "gemini_cli",
    }
}

/// Convert string to CLIToolType
fn string_to_cli_tool_type(s: &str) -> Result<crate::models::cli_tool::CLIToolType, String> {
    use crate::models::cli_tool::CLIToolType;
    match s {
        "claude_code" => Ok(CLIToolType::ClaudeCode),
        "codex" => Ok(CLIToolType::Codex),
        "gemini_cli" => Ok(CLIToolType::GeminiCli),
        _ => Err(format!("Unknown CLI tool type: {}", s)),
    }
}

/// Convert CLIAuthMode to database string
fn cli_auth_mode_to_string(mode: &crate::models::cli_tool::CLIAuthMode) -> &'static str {
    use crate::models::cli_tool::CLIAuthMode;
    match mode {
        CLIAuthMode::CliNative => "cli_native",
        CLIAuthMode::ApiKey => "api_key",
    }
}

/// Convert string to CLIAuthMode
fn string_to_cli_auth_mode(s: &str) -> crate::models::cli_tool::CLIAuthMode {
    use crate::models::cli_tool::CLIAuthMode;
    match s {
        "api_key" => CLIAuthMode::ApiKey,
        _ => CLIAuthMode::CliNative,
    }
}
