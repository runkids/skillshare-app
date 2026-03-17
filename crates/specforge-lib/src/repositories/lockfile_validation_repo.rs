// Lockfile Validation Config Repository
// Handles database operations for lockfile validation configuration

use chrono::Utc;
use rusqlite::params;

use crate::services::snapshot::validation::{
    BlockedPackageEntry, LockfileValidationConfig, ValidationRuleSet, ValidationStrictness,
    DEFAULT_ALLOWED_REGISTRIES,
};
use crate::utils::database::Database;

/// Repository for lockfile validation configuration
pub struct LockfileValidationRepository {
    db: Database,
}

impl LockfileValidationRepository {
    /// Create a new LockfileValidationRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Get the lockfile validation configuration
    pub fn get_config(&self) -> Result<LockfileValidationConfig, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT
                    enabled,
                    strictness,
                    require_integrity,
                    require_https_resolved,
                    check_allowed_registries,
                    check_blocked_packages,
                    check_manifest_consistency,
                    enhanced_typosquatting,
                    allowed_registries,
                    blocked_packages
                FROM lockfile_validation_config
                WHERE id = 1
                "#,
                [],
                |row| {
                    let enabled: i32 = row.get(0)?;
                    let strictness: String = row.get(1)?;
                    let require_integrity: i32 = row.get(2)?;
                    let require_https_resolved: i32 = row.get(3)?;
                    let check_allowed_registries: i32 = row.get(4)?;
                    let check_blocked_packages: i32 = row.get(5)?;
                    let check_manifest_consistency: i32 = row.get(6)?;
                    let enhanced_typosquatting: i32 = row.get(7)?;
                    let allowed_registries_json: String = row.get(8)?;
                    let blocked_packages_json: String = row.get(9)?;

                    Ok((
                        enabled,
                        strictness,
                        require_integrity,
                        require_https_resolved,
                        check_allowed_registries,
                        check_blocked_packages,
                        check_manifest_consistency,
                        enhanced_typosquatting,
                        allowed_registries_json,
                        blocked_packages_json,
                    ))
                },
            );

            match result {
                Ok((
                    enabled,
                    strictness,
                    require_integrity,
                    require_https_resolved,
                    check_allowed_registries,
                    check_blocked_packages,
                    check_manifest_consistency,
                    enhanced_typosquatting,
                    allowed_registries_json,
                    blocked_packages_json,
                )) => {
                    let allowed_registries: Vec<String> =
                        serde_json::from_str(&allowed_registries_json).unwrap_or_else(|_| {
                            DEFAULT_ALLOWED_REGISTRIES
                                .iter()
                                .map(|s| s.to_string())
                                .collect()
                        });

                    let blocked_packages: Vec<BlockedPackageEntry> =
                        serde_json::from_str(&blocked_packages_json).unwrap_or_default();

                    Ok(LockfileValidationConfig {
                        enabled: enabled != 0,
                        strictness: ValidationStrictness::from_str(&strictness)
                            .unwrap_or_default(),
                        rules: ValidationRuleSet {
                            require_integrity: require_integrity != 0,
                            require_https_resolved: require_https_resolved != 0,
                            check_allowed_registries: check_allowed_registries != 0,
                            check_blocked_packages: check_blocked_packages != 0,
                            check_manifest_consistency: check_manifest_consistency != 0,
                            enhanced_typosquatting: enhanced_typosquatting != 0,
                        },
                        allowed_registries,
                        blocked_packages,
                    })
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    // Return default config if not found
                    Ok(LockfileValidationConfig::default())
                }
                Err(e) => Err(format!("Failed to get lockfile validation config: {}", e)),
            }
        })
    }

    /// Save the lockfile validation configuration
    pub fn save_config(&self, config: &LockfileValidationConfig) -> Result<(), String> {
        let allowed_registries_json = serde_json::to_string(&config.allowed_registries)
            .map_err(|e| format!("Failed to serialize allowed_registries: {}", e))?;

        let blocked_packages_json = serde_json::to_string(&config.blocked_packages)
            .map_err(|e| format!("Failed to serialize blocked_packages: {}", e))?;

        let now = Utc::now().to_rfc3339();

        self.db.with_connection(|conn| {
            // Use INSERT ... ON CONFLICT DO UPDATE to avoid CASCADE issues
            conn.execute(
                r#"
                INSERT INTO lockfile_validation_config (
                    id,
                    enabled,
                    strictness,
                    require_integrity,
                    require_https_resolved,
                    check_allowed_registries,
                    check_blocked_packages,
                    check_manifest_consistency,
                    enhanced_typosquatting,
                    allowed_registries,
                    blocked_packages,
                    updated_at
                ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(id) DO UPDATE SET
                    enabled = excluded.enabled,
                    strictness = excluded.strictness,
                    require_integrity = excluded.require_integrity,
                    require_https_resolved = excluded.require_https_resolved,
                    check_allowed_registries = excluded.check_allowed_registries,
                    check_blocked_packages = excluded.check_blocked_packages,
                    check_manifest_consistency = excluded.check_manifest_consistency,
                    enhanced_typosquatting = excluded.enhanced_typosquatting,
                    allowed_registries = excluded.allowed_registries,
                    blocked_packages = excluded.blocked_packages,
                    updated_at = excluded.updated_at
                "#,
                params![
                    config.enabled as i32,
                    config.strictness.as_str(),
                    config.rules.require_integrity as i32,
                    config.rules.require_https_resolved as i32,
                    config.rules.check_allowed_registries as i32,
                    config.rules.check_blocked_packages as i32,
                    config.rules.check_manifest_consistency as i32,
                    config.rules.enhanced_typosquatting as i32,
                    allowed_registries_json,
                    blocked_packages_json,
                    now,
                ],
            )
            .map_err(|e| format!("Failed to save lockfile validation config: {}", e))?;

            Ok(())
        })
    }

    /// Enable or disable lockfile validation
    pub fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        let mut config = self.get_config()?;
        config.enabled = enabled;
        self.save_config(&config)
    }

    /// Update strictness level
    pub fn set_strictness(&self, strictness: ValidationStrictness) -> Result<(), String> {
        let mut config = self.get_config()?;
        config.strictness = strictness;
        self.save_config(&config)
    }

    /// Update validation rules
    pub fn set_rules(&self, rules: ValidationRuleSet) -> Result<(), String> {
        let mut config = self.get_config()?;
        config.rules = rules;
        self.save_config(&config)
    }

    /// Add a registry to the allowed list
    pub fn add_allowed_registry(&self, registry: &str) -> Result<(), String> {
        let mut config = self.get_config()?;
        if !config.allowed_registries.contains(&registry.to_string()) {
            config.allowed_registries.push(registry.to_string());
            self.save_config(&config)?;
        }
        Ok(())
    }

    /// Remove a registry from the allowed list
    pub fn remove_allowed_registry(&self, registry: &str) -> Result<(), String> {
        let mut config = self.get_config()?;
        config.allowed_registries.retain(|r| r != registry);
        self.save_config(&config)
    }

    /// Add a blocked package
    pub fn add_blocked_package(&self, entry: BlockedPackageEntry) -> Result<(), String> {
        let mut config = self.get_config()?;
        // Remove existing entry with same name if present
        config.blocked_packages.retain(|b| b.name != entry.name);
        config.blocked_packages.push(entry);
        self.save_config(&config)
    }

    /// Remove a blocked package
    pub fn remove_blocked_package(&self, package_name: &str) -> Result<(), String> {
        let mut config = self.get_config()?;
        config.blocked_packages.retain(|b| b.name != package_name);
        self.save_config(&config)
    }

    /// Reset to default configuration
    pub fn reset_to_defaults(&self) -> Result<LockfileValidationConfig, String> {
        let config = LockfileValidationConfig::default();
        self.save_config(&config)?;
        Ok(config)
    }
}
