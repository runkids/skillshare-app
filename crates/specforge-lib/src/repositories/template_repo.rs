// Template Repository
// Handles all database operations for custom step templates

use rusqlite::params;

use crate::models::step_template::{CustomStepTemplate, TemplateCategory};
use crate::utils::database::Database;

/// Repository for custom step template data access
pub struct TemplateRepository {
    db: Database,
}

impl TemplateRepository {
    /// Create a new TemplateRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// List all custom step templates
    pub fn list(&self) -> Result<Vec<CustomStepTemplate>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, name, command, category, description, is_custom, created_at
                    FROM custom_step_templates
                    ORDER BY name
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(TemplateRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        command: row.get(2)?,
                        category: row.get(3)?,
                        description: row.get(4)?,
                        is_custom: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                })
                .map_err(|e| format!("Failed to query templates: {}", e))?;

            let mut templates = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                templates.push(row.into_template());
            }

            Ok(templates)
        })
    }

    /// List templates by category
    pub fn list_by_category(&self, category: &str) -> Result<Vec<CustomStepTemplate>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, name, command, category, description, is_custom, created_at
                    FROM custom_step_templates
                    WHERE category = ?1
                    ORDER BY name
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![category], |row| {
                    Ok(TemplateRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        command: row.get(2)?,
                        category: row.get(3)?,
                        description: row.get(4)?,
                        is_custom: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                })
                .map_err(|e| format!("Failed to query templates: {}", e))?;

            let mut templates = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                templates.push(row.into_template());
            }

            Ok(templates)
        })
    }

    /// Get a template by ID
    pub fn get(&self, id: &str) -> Result<Option<CustomStepTemplate>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, name, command, category, description, is_custom, created_at
                FROM custom_step_templates
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(TemplateRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        command: row.get(2)?,
                        category: row.get(3)?,
                        description: row.get(4)?,
                        is_custom: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_template())),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get template: {}", e)),
            }
        })
    }

    /// Save a template
    pub fn save(&self, template: &CustomStepTemplate) -> Result<(), String> {
        let category_str = category_to_string(&template.category);

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO custom_step_templates
                (id, name, command, category, description, is_custom, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    template.id,
                    template.name,
                    template.command,
                    category_str,
                    template.description,
                    template.is_custom as i32,
                    template.created_at,
                ],
            )
            .map_err(|e| format!("Failed to save template: {}", e))?;

            Ok(())
        })
    }

    /// Delete a template by ID
    pub fn delete(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute(
                    "DELETE FROM custom_step_templates WHERE id = ?1",
                    params![id],
                )
                .map_err(|e| format!("Failed to delete template: {}", e))?;

            Ok(rows_affected > 0)
        })
    }
}

/// Convert TemplateCategory to string
fn category_to_string(category: &TemplateCategory) -> &'static str {
    match category {
        TemplateCategory::PackageManager => "package-manager",
        TemplateCategory::Git => "git",
        TemplateCategory::Docker => "docker",
        TemplateCategory::Shell => "shell",
        TemplateCategory::Testing => "testing",
        TemplateCategory::CodeQuality => "code-quality",
        TemplateCategory::Kubernetes => "kubernetes",
        TemplateCategory::Database => "database",
        TemplateCategory::Cloud => "cloud",
        TemplateCategory::Ai => "ai",
        TemplateCategory::Security => "security",
        TemplateCategory::Nodejs => "nodejs",
        TemplateCategory::Custom => "custom",
    }
}

/// Convert string to TemplateCategory
fn string_to_category(s: &str) -> TemplateCategory {
    match s {
        "package-manager" => TemplateCategory::PackageManager,
        "git" => TemplateCategory::Git,
        "docker" => TemplateCategory::Docker,
        "shell" => TemplateCategory::Shell,
        "testing" => TemplateCategory::Testing,
        "code-quality" => TemplateCategory::CodeQuality,
        "kubernetes" => TemplateCategory::Kubernetes,
        "database" => TemplateCategory::Database,
        "cloud" => TemplateCategory::Cloud,
        "ai" => TemplateCategory::Ai,
        "security" => TemplateCategory::Security,
        "nodejs" => TemplateCategory::Nodejs,
        "custom" | _ => TemplateCategory::Custom,
    }
}

/// Internal row structure for mapping database rows
struct TemplateRow {
    id: String,
    name: String,
    command: String,
    category: String,
    description: Option<String>,
    is_custom: i32,
    created_at: String,
}

impl TemplateRow {
    fn into_template(self) -> CustomStepTemplate {
        CustomStepTemplate {
            id: self.id,
            name: self.name,
            command: self.command,
            category: string_to_category(&self.category),
            description: self.description,
            is_custom: self.is_custom != 0,
            created_at: self.created_at,
        }
    }
}
