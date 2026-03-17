// SQLite Schema Definitions and Migrations
// Contains all table definitions and migration logic

use rusqlite::{Connection, params};

/// Current schema version
pub const CURRENT_VERSION: i32 = 8;

/// Migration struct containing version and SQL statements
struct Migration {
    version: i32,
    description: &'static str,
    up: &'static str,
}

/// All migrations in order
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "Initial schema - all tables",
        up: r#"
            -- Schema version tracking
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now')),
                description TEXT
            );

            -- Projects table
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                version TEXT DEFAULT '0.0.0',
                description TEXT,
                is_monorepo INTEGER DEFAULT 0,
                package_manager TEXT DEFAULT 'unknown' CHECK(package_manager IN ('npm', 'yarn', 'pnpm', 'bun', 'unknown')),
                scripts TEXT,
                worktree_sessions TEXT,
                monorepo_tool TEXT,
                framework TEXT,
                ui_framework TEXT,
                created_at TEXT NOT NULL,
                last_opened_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_projects_path ON projects(path);
            CREATE INDEX IF NOT EXISTS idx_projects_last_opened ON projects(last_opened_at DESC);

            -- Workflows table
            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
                nodes TEXT NOT NULL DEFAULT '[]',
                webhook TEXT,
                incoming_webhook TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_executed_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_workflows_project ON workflows(project_id);
            CREATE INDEX IF NOT EXISTS idx_workflows_updated ON workflows(updated_at DESC);

            -- Running executions (ephemeral, cleared on app restart)
            CREATE TABLE IF NOT EXISTS running_executions (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                execution_data TEXT NOT NULL
            );

            -- Execution history
            CREATE TABLE IF NOT EXISTS execution_history (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                workflow_name TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                finished_at TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                node_count INTEGER NOT NULL,
                completed_node_count INTEGER NOT NULL,
                error_message TEXT,
                output TEXT,
                triggered_by TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_execution_history_workflow ON execution_history(workflow_id);
            CREATE INDEX IF NOT EXISTS idx_execution_history_created ON execution_history(created_at DESC);

            -- Security scans
            CREATE TABLE IF NOT EXISTS security_scans (
                project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
                package_manager TEXT NOT NULL,
                last_scan TEXT,
                scan_history TEXT DEFAULT '[]',
                snooze_until TEXT
            );

            -- Custom step templates
            CREATE TABLE IF NOT EXISTS custom_step_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                command TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'custom',
                description TEXT,
                is_custom INTEGER DEFAULT 1,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_templates_category ON custom_step_templates(category);

            -- Settings (key-value store for flexibility)
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- MCP configuration (singleton)
            CREATE TABLE IF NOT EXISTS mcp_config (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                is_enabled INTEGER DEFAULT 0,
                permission_mode TEXT DEFAULT 'read_only' CHECK(permission_mode IN ('read_only', 'execute_with_confirm', 'full_access')),
                allowed_tools TEXT DEFAULT '[]',
                log_requests INTEGER DEFAULT 1,
                encrypted_secrets TEXT
            );
            INSERT OR IGNORE INTO mcp_config (id) VALUES (1);

            -- MCP request logs
            CREATE TABLE IF NOT EXISTS mcp_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                tool TEXT NOT NULL,
                arguments TEXT NOT NULL DEFAULT '{}',
                result TEXT NOT NULL,
                duration_ms INTEGER NOT NULL DEFAULT 0,
                error TEXT,
                source TEXT DEFAULT 'mcp_server'
            );
            CREATE INDEX IF NOT EXISTS idx_mcp_logs_timestamp ON mcp_logs(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_mcp_logs_tool ON mcp_logs(tool);

            -- AI providers
            CREATE TABLE IF NOT EXISTS ai_providers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                provider TEXT NOT NULL CHECK(provider IN ('openai', 'anthropic', 'gemini', 'ollama', 'lm_studio')),
                endpoint TEXT NOT NULL,
                model TEXT NOT NULL,
                is_default INTEGER DEFAULT 0,
                is_enabled INTEGER DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- AI prompt templates
            CREATE TABLE IF NOT EXISTS ai_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                category TEXT NOT NULL DEFAULT 'git_commit' CHECK(category IN (
                    'git_commit', 'pull_request', 'code_review',
                    'documentation', 'release_notes', 'security_advisory', 'custom'
                )),
                template TEXT NOT NULL,
                output_format TEXT,
                is_default INTEGER DEFAULT 0,
                is_builtin INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_ai_templates_category ON ai_templates(category);

            -- Project-specific AI settings
            CREATE TABLE IF NOT EXISTS project_ai_settings (
                project_path TEXT PRIMARY KEY,
                preferred_provider_id TEXT REFERENCES ai_providers(id) ON DELETE SET NULL,
                preferred_template_id TEXT REFERENCES ai_templates(id) ON DELETE SET NULL
            );

            -- AI API keys (encrypted)
            CREATE TABLE IF NOT EXISTS ai_api_keys (
                provider_id TEXT PRIMARY KEY REFERENCES ai_providers(id) ON DELETE CASCADE,
                ciphertext TEXT NOT NULL,
                nonce TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- Deploy accounts
            CREATE TABLE IF NOT EXISTS deploy_accounts (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL CHECK(platform IN ('github_pages', 'netlify', 'cloudflare_pages')),
                platform_user_id TEXT NOT NULL,
                username TEXT NOT NULL,
                display_name TEXT,
                avatar_url TEXT,
                access_token TEXT NOT NULL,
                connected_at TEXT NOT NULL,
                expires_at TEXT,
                UNIQUE(platform, platform_user_id)
            );
            CREATE INDEX IF NOT EXISTS idx_deploy_accounts_platform ON deploy_accounts(platform);

            -- Deploy account tokens (encrypted)
            CREATE TABLE IF NOT EXISTS deploy_account_tokens (
                account_id TEXT PRIMARY KEY REFERENCES deploy_accounts(id) ON DELETE CASCADE,
                ciphertext TEXT NOT NULL,
                nonce TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- Deploy preferences (singleton)
            CREATE TABLE IF NOT EXISTS deploy_preferences (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                default_github_pages_account_id TEXT REFERENCES deploy_accounts(id) ON DELETE SET NULL,
                default_netlify_account_id TEXT REFERENCES deploy_accounts(id) ON DELETE SET NULL,
                default_cloudflare_pages_account_id TEXT REFERENCES deploy_accounts(id) ON DELETE SET NULL
            );
            INSERT OR IGNORE INTO deploy_preferences (id) VALUES (1);

            -- Deployment configurations per project
            CREATE TABLE IF NOT EXISTS deployment_configs (
                project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
                platform TEXT NOT NULL,
                account_id TEXT REFERENCES deploy_accounts(id) ON DELETE SET NULL,
                environment TEXT DEFAULT 'production',
                framework_preset TEXT,
                env_variables TEXT DEFAULT '[]',
                root_directory TEXT,
                install_command TEXT,
                build_command TEXT,
                output_directory TEXT,
                netlify_site_id TEXT,
                netlify_site_name TEXT,
                cloudflare_account_id TEXT,
                cloudflare_project_name TEXT
            );

            -- Deployment history
            CREATE TABLE IF NOT EXISTS deployments (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                status TEXT NOT NULL,
                url TEXT,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                commit_hash TEXT,
                commit_message TEXT,
                error_message TEXT,
                admin_url TEXT,
                deploy_time INTEGER,
                branch TEXT,
                site_name TEXT,
                preview_url TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_deployments_project ON deployments(project_id);
            CREATE INDEX IF NOT EXISTS idx_deployments_created ON deployments(created_at DESC);

            -- Webhook tokens (encrypted)
            CREATE TABLE IF NOT EXISTS webhook_tokens (
                workflow_id TEXT PRIMARY KEY REFERENCES workflows(id) ON DELETE CASCADE,
                ciphertext TEXT NOT NULL,
                nonce TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- CLI tools configuration
            CREATE TABLE IF NOT EXISTS cli_tools (
                id TEXT PRIMARY KEY,
                tool_type TEXT NOT NULL CHECK(tool_type IN ('claude_code', 'codex', 'gemini_cli')),
                name TEXT NOT NULL,
                binary_path TEXT,
                is_enabled INTEGER DEFAULT 1,
                auth_mode TEXT NOT NULL DEFAULT 'cli_native' CHECK(auth_mode IN ('cli_native', 'api_key')),
                api_key_provider_id TEXT REFERENCES ai_providers(id) ON DELETE SET NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_cli_tools_type ON cli_tools(tool_type);
            CREATE INDEX IF NOT EXISTS idx_cli_tools_enabled ON cli_tools(is_enabled);

            -- CLI execution logs
            CREATE TABLE IF NOT EXISTS cli_execution_logs (
                id TEXT PRIMARY KEY,
                tool_type TEXT NOT NULL CHECK(tool_type IN ('claude_code', 'codex', 'gemini_cli')),
                project_path TEXT,
                prompt_hash TEXT NOT NULL,
                model TEXT,
                execution_time_ms INTEGER,
                exit_code INTEGER,
                tokens_used INTEGER,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_cli_logs_created ON cli_execution_logs(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_cli_logs_project ON cli_execution_logs(project_path);
            CREATE INDEX IF NOT EXISTS idx_cli_logs_tool ON cli_execution_logs(tool_type);

            -- Notifications history
            CREATE TABLE IF NOT EXISTS notifications (
                id TEXT PRIMARY KEY,
                notification_type TEXT NOT NULL,
                category TEXT NOT NULL CHECK(category IN (
                    'webhooks', 'workflow_execution', 'git_operations',
                    'security_scans', 'deployments'
                )),
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                is_read INTEGER DEFAULT 0,
                metadata TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_notifications_created ON notifications(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_notifications_unread ON notifications(is_read, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_notifications_category ON notifications(category);

            -- MCP action definitions
            CREATE TABLE IF NOT EXISTS mcp_actions (
                id TEXT PRIMARY KEY,
                action_type TEXT NOT NULL CHECK(action_type IN ('script', 'webhook', 'workflow')),
                name TEXT NOT NULL,
                description TEXT,
                config TEXT NOT NULL,
                project_id TEXT,
                is_enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_mcp_actions_project ON mcp_actions(project_id);
            CREATE INDEX IF NOT EXISTS idx_mcp_actions_type ON mcp_actions(action_type);

            -- MCP action permission rules
            CREATE TABLE IF NOT EXISTS mcp_action_permissions (
                id TEXT PRIMARY KEY,
                action_id TEXT,
                action_type TEXT CHECK(action_type IN ('script', 'webhook', 'workflow') OR action_type IS NULL),
                permission_level TEXT NOT NULL CHECK(permission_level IN ('require_confirm', 'auto_approve', 'deny')),
                created_at TEXT NOT NULL,
                FOREIGN KEY (action_id) REFERENCES mcp_actions(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_mcp_permissions_action ON mcp_action_permissions(action_id);
            CREATE INDEX IF NOT EXISTS idx_mcp_permissions_type ON mcp_action_permissions(action_type);

            -- MCP action execution history
            CREATE TABLE IF NOT EXISTS mcp_action_executions (
                id TEXT PRIMARY KEY,
                action_id TEXT,
                action_type TEXT NOT NULL,
                action_name TEXT NOT NULL,
                source_client TEXT,
                parameters TEXT,
                status TEXT NOT NULL CHECK(status IN ('pending_confirm', 'queued', 'running', 'completed', 'failed', 'cancelled', 'timed_out')),
                result TEXT,
                error_message TEXT,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                duration_ms INTEGER,
                FOREIGN KEY (action_id) REFERENCES mcp_actions(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_mcp_executions_action ON mcp_action_executions(action_id);
            CREATE INDEX IF NOT EXISTS idx_mcp_executions_status ON mcp_action_executions(status);
            CREATE INDEX IF NOT EXISTS idx_mcp_executions_started ON mcp_action_executions(started_at DESC);

            -- AI Assistant Conversations
            CREATE TABLE IF NOT EXISTS ai_conversations (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT,
                project_path TEXT,
                provider_id TEXT,
                message_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (provider_id) REFERENCES ai_providers(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_ai_conversations_updated ON ai_conversations(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_ai_conversations_project ON ai_conversations(project_path);

            -- AI Assistant Messages
            CREATE TABLE IF NOT EXISTS ai_messages (
                id TEXT PRIMARY KEY NOT NULL,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
                content TEXT NOT NULL,
                tool_calls TEXT,
                tool_results TEXT,
                status TEXT NOT NULL CHECK (status IN ('pending', 'sent', 'error')) DEFAULT 'sent',
                tokens_used INTEGER,
                model TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES ai_conversations(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_ai_messages_conversation ON ai_messages(conversation_id, created_at);
        "#,
    },
    Migration {
        version: 2,
        description: "Add dev_server_mode to mcp_config",
        up: r#"
            ALTER TABLE mcp_config ADD COLUMN dev_server_mode TEXT DEFAULT 'mcp_managed' CHECK(dev_server_mode IN ('mcp_managed', 'reject_with_hint'));
        "#,
    },
    Migration {
        version: 3,
        description: "Add ui_integrated option to dev_server_mode",
        up: r#"
            -- SQLite requires recreating the table to modify CHECK constraints
            -- Create new table with updated constraint
            CREATE TABLE mcp_config_new (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                is_enabled INTEGER DEFAULT 0,
                permission_mode TEXT DEFAULT 'read_only' CHECK(permission_mode IN ('read_only', 'execute_with_confirm', 'full_access')),
                dev_server_mode TEXT DEFAULT 'mcp_managed' CHECK(dev_server_mode IN ('mcp_managed', 'ui_integrated', 'reject_with_hint')),
                allowed_tools TEXT DEFAULT '[]',
                log_requests INTEGER DEFAULT 1,
                encrypted_secrets TEXT
            );

            -- Copy existing data
            INSERT INTO mcp_config_new (id, is_enabled, permission_mode, dev_server_mode, allowed_tools, log_requests, encrypted_secrets)
            SELECT id, is_enabled, permission_mode, dev_server_mode, allowed_tools, log_requests, encrypted_secrets
            FROM mcp_config;

            -- Drop old table and rename new one
            DROP TABLE mcp_config;
            ALTER TABLE mcp_config_new RENAME TO mcp_config;
        "#,
    },
    Migration {
        version: 4,
        description: "Time Machine - Execution snapshots and security insights",
        up: r#"
            -- Execution snapshots table
            CREATE TABLE IF NOT EXISTS execution_snapshots (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                execution_id TEXT NOT NULL,
                project_path TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('capturing', 'completed', 'failed')),
                lockfile_type TEXT CHECK(lockfile_type IN ('npm', 'pnpm', 'yarn', 'bun')),
                lockfile_hash TEXT,
                dependency_tree_hash TEXT,
                package_json_hash TEXT,
                total_dependencies INTEGER DEFAULT 0,
                direct_dependencies INTEGER DEFAULT 0,
                dev_dependencies INTEGER DEFAULT 0,
                security_score INTEGER,
                postinstall_count INTEGER DEFAULT 0,
                storage_path TEXT,
                compressed_size INTEGER,
                execution_duration_ms INTEGER,
                error_message TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_snapshots_workflow ON execution_snapshots(workflow_id);
            CREATE INDEX IF NOT EXISTS idx_snapshots_project ON execution_snapshots(project_path);
            CREATE INDEX IF NOT EXISTS idx_snapshots_created ON execution_snapshots(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_snapshots_execution ON execution_snapshots(execution_id);

            -- Snapshot dependencies (denormalized for fast queries)
            CREATE TABLE IF NOT EXISTS snapshot_dependencies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id TEXT NOT NULL,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                is_direct INTEGER DEFAULT 0,
                is_dev INTEGER DEFAULT 0,
                has_postinstall INTEGER DEFAULT 0,
                postinstall_script TEXT,
                integrity_hash TEXT,
                resolved_url TEXT,
                FOREIGN KEY (snapshot_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_snapshot_deps_snapshot ON snapshot_dependencies(snapshot_id);
            CREATE INDEX IF NOT EXISTS idx_snapshot_deps_name ON snapshot_dependencies(name);
            CREATE INDEX IF NOT EXISTS idx_snapshot_deps_postinstall ON snapshot_dependencies(has_postinstall) WHERE has_postinstall = 1;

            -- FTS5 virtual table for dependency search
            CREATE VIRTUAL TABLE IF NOT EXISTS snapshot_dependencies_fts USING fts5(
                name,
                version,
                content=snapshot_dependencies,
                content_rowid=id
            );

            -- Triggers to keep FTS in sync
            CREATE TRIGGER IF NOT EXISTS snapshot_deps_ai AFTER INSERT ON snapshot_dependencies BEGIN
                INSERT INTO snapshot_dependencies_fts(rowid, name, version) VALUES (new.id, new.name, new.version);
            END;
            CREATE TRIGGER IF NOT EXISTS snapshot_deps_ad AFTER DELETE ON snapshot_dependencies BEGIN
                INSERT INTO snapshot_dependencies_fts(snapshot_dependencies_fts, rowid, name, version) VALUES('delete', old.id, old.name, old.version);
            END;
            CREATE TRIGGER IF NOT EXISTS snapshot_deps_au AFTER UPDATE ON snapshot_dependencies BEGIN
                INSERT INTO snapshot_dependencies_fts(snapshot_dependencies_fts, rowid, name, version) VALUES('delete', old.id, old.name, old.version);
                INSERT INTO snapshot_dependencies_fts(rowid, name, version) VALUES (new.id, new.name, new.version);
            END;

            -- Security insights table
            CREATE TABLE IF NOT EXISTS security_insights (
                id TEXT PRIMARY KEY,
                snapshot_id TEXT NOT NULL,
                insight_type TEXT NOT NULL CHECK(insight_type IN (
                    'new_dependency', 'removed_dependency', 'version_change',
                    'postinstall_added', 'postinstall_removed', 'postinstall_changed',
                    'integrity_mismatch', 'typosquatting_suspect', 'frequent_updater',
                    'suspicious_script'
                )),
                severity TEXT NOT NULL CHECK(severity IN ('info', 'low', 'medium', 'high', 'critical')),
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                package_name TEXT,
                previous_value TEXT,
                current_value TEXT,
                recommendation TEXT,
                metadata TEXT,
                is_dismissed INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                FOREIGN KEY (snapshot_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_insights_snapshot ON security_insights(snapshot_id);
            CREATE INDEX IF NOT EXISTS idx_insights_type ON security_insights(insight_type);
            CREATE INDEX IF NOT EXISTS idx_insights_severity ON security_insights(severity);
            CREATE INDEX IF NOT EXISTS idx_insights_package ON security_insights(package_name);

            -- Snapshot diff cache for performance
            CREATE TABLE IF NOT EXISTS snapshot_diff_cache (
                id TEXT PRIMARY KEY,
                snapshot_a_id TEXT NOT NULL,
                snapshot_b_id TEXT NOT NULL,
                diff_data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (snapshot_a_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE,
                FOREIGN KEY (snapshot_b_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE,
                UNIQUE(snapshot_a_id, snapshot_b_id)
            );
            CREATE INDEX IF NOT EXISTS idx_diff_cache_snapshots ON snapshot_diff_cache(snapshot_a_id, snapshot_b_id);
        "#,
    },
    Migration {
        version: 5,
        description: "Fix execution_snapshots schema - recreate tables with correct columns",
        up: r#"
            -- Drop FTS triggers first
            DROP TRIGGER IF EXISTS snapshot_deps_ai;
            DROP TRIGGER IF EXISTS snapshot_deps_ad;
            DROP TRIGGER IF EXISTS snapshot_deps_au;

            -- Drop FTS table
            DROP TABLE IF EXISTS snapshot_dependencies_fts;

            -- Drop tables in correct order (respecting foreign keys)
            DROP TABLE IF EXISTS snapshot_diff_cache;
            DROP TABLE IF EXISTS security_insights;
            DROP TABLE IF EXISTS snapshot_dependencies;
            DROP TABLE IF EXISTS execution_snapshots;

            -- Recreate execution_snapshots with correct schema
            CREATE TABLE execution_snapshots (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                execution_id TEXT NOT NULL,
                project_path TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('capturing', 'completed', 'failed')),
                lockfile_type TEXT CHECK(lockfile_type IN ('npm', 'pnpm', 'yarn', 'bun')),
                lockfile_hash TEXT,
                dependency_tree_hash TEXT,
                package_json_hash TEXT,
                total_dependencies INTEGER DEFAULT 0,
                direct_dependencies INTEGER DEFAULT 0,
                dev_dependencies INTEGER DEFAULT 0,
                security_score INTEGER,
                postinstall_count INTEGER DEFAULT 0,
                storage_path TEXT,
                compressed_size INTEGER,
                execution_duration_ms INTEGER,
                error_message TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_snapshots_workflow ON execution_snapshots(workflow_id);
            CREATE INDEX idx_snapshots_project ON execution_snapshots(project_path);
            CREATE INDEX idx_snapshots_created ON execution_snapshots(created_at DESC);
            CREATE INDEX idx_snapshots_execution ON execution_snapshots(execution_id);

            -- Recreate snapshot_dependencies
            CREATE TABLE snapshot_dependencies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id TEXT NOT NULL,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                is_direct INTEGER DEFAULT 0,
                is_dev INTEGER DEFAULT 0,
                has_postinstall INTEGER DEFAULT 0,
                postinstall_script TEXT,
                integrity_hash TEXT,
                resolved_url TEXT,
                FOREIGN KEY (snapshot_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_snapshot_deps_snapshot ON snapshot_dependencies(snapshot_id);
            CREATE INDEX idx_snapshot_deps_name ON snapshot_dependencies(name);
            CREATE INDEX idx_snapshot_deps_postinstall ON snapshot_dependencies(has_postinstall) WHERE has_postinstall = 1;

            -- Recreate FTS5 virtual table
            CREATE VIRTUAL TABLE snapshot_dependencies_fts USING fts5(
                name,
                version,
                content=snapshot_dependencies,
                content_rowid=id
            );

            -- Recreate triggers
            CREATE TRIGGER snapshot_deps_ai AFTER INSERT ON snapshot_dependencies BEGIN
                INSERT INTO snapshot_dependencies_fts(rowid, name, version) VALUES (new.id, new.name, new.version);
            END;
            CREATE TRIGGER snapshot_deps_ad AFTER DELETE ON snapshot_dependencies BEGIN
                INSERT INTO snapshot_dependencies_fts(snapshot_dependencies_fts, rowid, name, version) VALUES('delete', old.id, old.name, old.version);
            END;
            CREATE TRIGGER snapshot_deps_au AFTER UPDATE ON snapshot_dependencies BEGIN
                INSERT INTO snapshot_dependencies_fts(snapshot_dependencies_fts, rowid, name, version) VALUES('delete', old.id, old.name, old.version);
                INSERT INTO snapshot_dependencies_fts(rowid, name, version) VALUES (new.id, new.name, new.version);
            END;

            -- Recreate security_insights
            CREATE TABLE security_insights (
                id TEXT PRIMARY KEY,
                snapshot_id TEXT NOT NULL,
                insight_type TEXT NOT NULL CHECK(insight_type IN (
                    'new_dependency', 'removed_dependency', 'version_change',
                    'postinstall_added', 'postinstall_removed', 'postinstall_changed',
                    'integrity_mismatch', 'typosquatting_suspect', 'frequent_updater',
                    'suspicious_script'
                )),
                severity TEXT NOT NULL CHECK(severity IN ('info', 'low', 'medium', 'high', 'critical')),
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                package_name TEXT,
                previous_value TEXT,
                current_value TEXT,
                recommendation TEXT,
                metadata TEXT,
                is_dismissed INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                FOREIGN KEY (snapshot_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_insights_snapshot ON security_insights(snapshot_id);
            CREATE INDEX idx_insights_type ON security_insights(insight_type);
            CREATE INDEX idx_insights_severity ON security_insights(severity);
            CREATE INDEX idx_insights_package ON security_insights(package_name);

            -- Recreate snapshot_diff_cache
            CREATE TABLE snapshot_diff_cache (
                id TEXT PRIMARY KEY,
                snapshot_a_id TEXT NOT NULL,
                snapshot_b_id TEXT NOT NULL,
                diff_data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (snapshot_a_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE,
                FOREIGN KEY (snapshot_b_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE,
                UNIQUE(snapshot_a_id, snapshot_b_id)
            );
            CREATE INDEX idx_diff_cache_snapshots ON snapshot_diff_cache(snapshot_a_id, snapshot_b_id);
        "#,
    },
    Migration {
        version: 6,
        description: "Time Machine - Project-level snapshots (remove workflow binding)",
        up: r#"
            -- Drop FTS triggers first
            DROP TRIGGER IF EXISTS snapshot_deps_ai;
            DROP TRIGGER IF EXISTS snapshot_deps_ad;
            DROP TRIGGER IF EXISTS snapshot_deps_au;

            -- Drop FTS table
            DROP TABLE IF EXISTS snapshot_dependencies_fts;

            -- Drop dependent tables
            DROP TABLE IF EXISTS snapshot_diff_cache;
            DROP TABLE IF EXISTS security_insights;
            DROP TABLE IF EXISTS snapshot_dependencies;

            -- Recreate execution_snapshots without workflow_id, execution_id, execution_duration_ms
            CREATE TABLE execution_snapshots_new (
                id TEXT PRIMARY KEY,
                project_path TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('capturing', 'completed', 'failed')),
                trigger_source TEXT NOT NULL DEFAULT 'lockfile_change' CHECK(trigger_source IN ('lockfile_change', 'manual')),
                lockfile_type TEXT CHECK(lockfile_type IN ('npm', 'pnpm', 'yarn', 'bun')),
                lockfile_hash TEXT,
                dependency_tree_hash TEXT,
                package_json_hash TEXT,
                total_dependencies INTEGER DEFAULT 0,
                direct_dependencies INTEGER DEFAULT 0,
                dev_dependencies INTEGER DEFAULT 0,
                security_score INTEGER,
                postinstall_count INTEGER DEFAULT 0,
                storage_path TEXT,
                compressed_size INTEGER,
                error_message TEXT,
                created_at TEXT NOT NULL
            );

            -- Migrate existing data (drop workflow_id, execution_id, execution_duration_ms)
            INSERT INTO execution_snapshots_new (
                id, project_path, status, trigger_source, lockfile_type, lockfile_hash,
                dependency_tree_hash, package_json_hash, total_dependencies, direct_dependencies,
                dev_dependencies, security_score, postinstall_count, storage_path, compressed_size,
                error_message, created_at
            )
            SELECT
                id, project_path, status, 'lockfile_change', lockfile_type, lockfile_hash,
                dependency_tree_hash, package_json_hash, total_dependencies, direct_dependencies,
                dev_dependencies, security_score, postinstall_count, storage_path, compressed_size,
                error_message, created_at
            FROM execution_snapshots;

            DROP TABLE execution_snapshots;
            ALTER TABLE execution_snapshots_new RENAME TO execution_snapshots;

            -- Recreate indexes (without workflow index)
            CREATE INDEX idx_snapshots_project ON execution_snapshots(project_path);
            CREATE INDEX idx_snapshots_created ON execution_snapshots(created_at DESC);
            CREATE INDEX idx_snapshots_hash ON execution_snapshots(project_path, lockfile_hash);
            CREATE INDEX idx_snapshots_trigger ON execution_snapshots(trigger_source);

            -- Recreate snapshot_dependencies
            CREATE TABLE snapshot_dependencies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id TEXT NOT NULL,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                is_direct INTEGER DEFAULT 0,
                is_dev INTEGER DEFAULT 0,
                has_postinstall INTEGER DEFAULT 0,
                postinstall_script TEXT,
                integrity_hash TEXT,
                resolved_url TEXT,
                FOREIGN KEY (snapshot_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_snapshot_deps_snapshot ON snapshot_dependencies(snapshot_id);
            CREATE INDEX idx_snapshot_deps_name ON snapshot_dependencies(name);
            CREATE INDEX idx_snapshot_deps_postinstall ON snapshot_dependencies(has_postinstall) WHERE has_postinstall = 1;

            -- Recreate FTS5 virtual table
            CREATE VIRTUAL TABLE snapshot_dependencies_fts USING fts5(
                name,
                version,
                content=snapshot_dependencies,
                content_rowid=id
            );

            -- Recreate triggers
            CREATE TRIGGER snapshot_deps_ai AFTER INSERT ON snapshot_dependencies BEGIN
                INSERT INTO snapshot_dependencies_fts(rowid, name, version) VALUES (new.id, new.name, new.version);
            END;
            CREATE TRIGGER snapshot_deps_ad AFTER DELETE ON snapshot_dependencies BEGIN
                INSERT INTO snapshot_dependencies_fts(snapshot_dependencies_fts, rowid, name, version) VALUES('delete', old.id, old.name, old.version);
            END;
            CREATE TRIGGER snapshot_deps_au AFTER UPDATE ON snapshot_dependencies BEGIN
                INSERT INTO snapshot_dependencies_fts(snapshot_dependencies_fts, rowid, name, version) VALUES('delete', old.id, old.name, old.version);
                INSERT INTO snapshot_dependencies_fts(rowid, name, version) VALUES (new.id, new.name, new.version);
            END;

            -- Recreate security_insights
            CREATE TABLE security_insights (
                id TEXT PRIMARY KEY,
                snapshot_id TEXT NOT NULL,
                insight_type TEXT NOT NULL CHECK(insight_type IN (
                    'new_dependency', 'removed_dependency', 'version_change',
                    'postinstall_added', 'postinstall_removed', 'postinstall_changed',
                    'integrity_mismatch', 'typosquatting_suspect', 'frequent_updater',
                    'suspicious_script'
                )),
                severity TEXT NOT NULL CHECK(severity IN ('info', 'low', 'medium', 'high', 'critical')),
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                package_name TEXT,
                previous_value TEXT,
                current_value TEXT,
                recommendation TEXT,
                metadata TEXT,
                is_dismissed INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                FOREIGN KEY (snapshot_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_insights_snapshot ON security_insights(snapshot_id);
            CREATE INDEX idx_insights_type ON security_insights(insight_type);
            CREATE INDEX idx_insights_severity ON security_insights(severity);
            CREATE INDEX idx_insights_package ON security_insights(package_name);

            -- Recreate snapshot_diff_cache
            CREATE TABLE snapshot_diff_cache (
                id TEXT PRIMARY KEY,
                snapshot_a_id TEXT NOT NULL,
                snapshot_b_id TEXT NOT NULL,
                diff_data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (snapshot_a_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE,
                FOREIGN KEY (snapshot_b_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE,
                UNIQUE(snapshot_a_id, snapshot_b_id)
            );
            CREATE INDEX idx_diff_cache_snapshots ON snapshot_diff_cache(snapshot_a_id, snapshot_b_id);

            -- New table: Track lockfile state per project (for change detection)
            CREATE TABLE project_lockfile_state (
                project_path TEXT PRIMARY KEY,
                lockfile_type TEXT CHECK(lockfile_type IN ('npm', 'pnpm', 'yarn', 'bun')),
                lockfile_hash TEXT NOT NULL,
                last_snapshot_id TEXT,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (last_snapshot_id) REFERENCES execution_snapshots(id) ON DELETE SET NULL
            );

            -- New table: Time Machine settings
            CREATE TABLE time_machine_settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                auto_watch_enabled INTEGER DEFAULT 1,
                debounce_ms INTEGER DEFAULT 2000,
                updated_at TEXT NOT NULL
            );

            -- Initialize default settings
            INSERT INTO time_machine_settings (id, auto_watch_enabled, debounce_ms, updated_at)
            VALUES (1, 1, 2000, datetime('now'));
        "#,
    },
    Migration {
        version: 7,
        description: "Add lockfile validation insight types to security_insights",
        up: r#"
            -- SQLite doesn't support ALTER TABLE to modify CHECK constraints
            -- We need to recreate the table with the new constraint

            -- Create temp table with new constraint
            CREATE TABLE security_insights_new (
                id TEXT PRIMARY KEY,
                snapshot_id TEXT NOT NULL,
                insight_type TEXT NOT NULL CHECK(insight_type IN (
                    'new_dependency', 'removed_dependency', 'version_change',
                    'postinstall_added', 'postinstall_removed', 'postinstall_changed',
                    'integrity_mismatch', 'typosquatting_suspect', 'frequent_updater',
                    'suspicious_script',
                    'insecure_protocol', 'unexpected_registry', 'manifest_mismatch',
                    'blocked_package', 'missing_integrity', 'scope_confusion', 'homoglyph_suspect'
                )),
                severity TEXT NOT NULL CHECK(severity IN ('info', 'low', 'medium', 'high', 'critical')),
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                package_name TEXT,
                previous_value TEXT,
                current_value TEXT,
                recommendation TEXT,
                metadata TEXT,
                is_dismissed INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                FOREIGN KEY (snapshot_id) REFERENCES execution_snapshots(id) ON DELETE CASCADE
            );

            -- Copy existing data
            INSERT INTO security_insights_new SELECT * FROM security_insights;

            -- Drop old table
            DROP TABLE security_insights;

            -- Rename new table
            ALTER TABLE security_insights_new RENAME TO security_insights;

            -- Recreate indexes
            CREATE INDEX idx_insights_snapshot ON security_insights(snapshot_id);
            CREATE INDEX idx_insights_type ON security_insights(insight_type);
            CREATE INDEX idx_insights_severity ON security_insights(severity);
            CREATE INDEX idx_insights_package ON security_insights(package_name);

            -- Create lockfile_validation_config table for storing validation settings
            CREATE TABLE IF NOT EXISTS lockfile_validation_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                enabled INTEGER DEFAULT 0,
                strictness TEXT DEFAULT 'standard' CHECK(strictness IN ('relaxed', 'standard', 'strict')),
                require_integrity INTEGER DEFAULT 1,
                require_https_resolved INTEGER DEFAULT 1,
                check_allowed_registries INTEGER DEFAULT 0,
                check_blocked_packages INTEGER DEFAULT 1,
                check_manifest_consistency INTEGER DEFAULT 1,
                enhanced_typosquatting INTEGER DEFAULT 0,
                allowed_registries TEXT DEFAULT '[]',
                blocked_packages TEXT DEFAULT '[]',
                updated_at TEXT NOT NULL
            );

            -- Initialize default validation config
            INSERT OR IGNORE INTO lockfile_validation_config (id, updated_at)
            VALUES (1, datetime('now'));
        "#,
    },
    Migration {
        version: 8,
        description: "Security Audit Log - Structured security event logging",
        up: r#"
            -- Security audit log table
            CREATE TABLE IF NOT EXISTS security_audit_log (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL CHECK(event_type IN (
                    'authentication', 'authorization', 'data_access',
                    'configuration', 'tool_execution', 'webhook_trigger', 'security_alert'
                )),
                actor_type TEXT NOT NULL CHECK(actor_type IN ('user', 'ai_assistant', 'webhook', 'system')),
                actor_id TEXT,
                action TEXT NOT NULL,
                resource_type TEXT,
                resource_id TEXT,
                resource_name TEXT,
                outcome TEXT NOT NULL CHECK(outcome IN ('success', 'failure', 'denied')),
                outcome_reason TEXT,
                details TEXT,
                client_ip TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- Indexes for common queries
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON security_audit_log(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_audit_event_type ON security_audit_log(event_type);
            CREATE INDEX IF NOT EXISTS idx_audit_actor ON security_audit_log(actor_type, actor_id);
            CREATE INDEX IF NOT EXISTS idx_audit_resource ON security_audit_log(resource_type, resource_id);
            CREATE INDEX IF NOT EXISTS idx_audit_outcome ON security_audit_log(outcome);

            -- Auto-cleanup trigger: delete logs older than 90 days on each insert
            CREATE TRIGGER IF NOT EXISTS cleanup_old_audit_logs
            AFTER INSERT ON security_audit_log
            BEGIN
                DELETE FROM security_audit_log
                WHERE timestamp < datetime('now', '-90 days');
            END;
        "#,
    },
];

/// Run all pending migrations using Database wrapper
pub fn migrate(db: &super::database::Database) -> Result<(), String> {
    db.with_connection(|conn| run_migrations(conn))
}

/// Run all pending migrations
pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    // Ensure schema_version table exists first
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now')),
            description TEXT
        )
        "#,
        [],
    )
    .map_err(|e| format!("Failed to create schema_version table: {}", e))?;

    // Get current version
    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Run pending migrations
    for migration in MIGRATIONS {
        if migration.version > current_version {
            log::info!(
                "Running migration v{}: {}",
                migration.version,
                migration.description
            );

            // Execute migration SQL
            conn.execute_batch(migration.up)
                .map_err(|e| format!("Migration v{} failed: {}", migration.version, e))?;

            // Record migration
            conn.execute(
                "INSERT INTO schema_version (version, description) VALUES (?1, ?2)",
                params![migration.version, migration.description],
            )
            .map_err(|e| format!("Failed to record migration v{}: {}", migration.version, e))?;

            log::info!("Migration v{} completed", migration.version);
        }
    }

    Ok(())
}

/// Get the current schema version
pub fn get_version(conn: &Connection) -> Result<i32, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to get schema version: {}", e))
}

/// Check if a table exists
pub fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, String> {
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            params![table_name],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to check table existence: {}", e))?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // Verify schema version
        let version = get_version(&conn).unwrap();
        assert_eq!(version, CURRENT_VERSION);

        // Verify tables exist
        assert!(table_exists(&conn, "projects").unwrap());
        assert!(table_exists(&conn, "workflows").unwrap());
        assert!(table_exists(&conn, "settings").unwrap());
        assert!(table_exists(&conn, "mcp_config").unwrap());
        assert!(table_exists(&conn, "ai_providers").unwrap());
        assert!(table_exists(&conn, "ai_templates").unwrap());
        assert!(table_exists(&conn, "ai_api_keys").unwrap());
        assert!(table_exists(&conn, "deploy_accounts").unwrap());
        assert!(table_exists(&conn, "deploy_account_tokens").unwrap());
        assert!(table_exists(&conn, "webhook_tokens").unwrap());
        assert!(table_exists(&conn, "notifications").unwrap());
        assert!(table_exists(&conn, "mcp_actions").unwrap());
        assert!(table_exists(&conn, "cli_tools").unwrap());
        assert!(table_exists(&conn, "ai_conversations").unwrap());
        assert!(table_exists(&conn, "ai_messages").unwrap());
        // Time Machine tables (v4)
        assert!(table_exists(&conn, "execution_snapshots").unwrap());
        assert!(table_exists(&conn, "snapshot_dependencies").unwrap());
        assert!(table_exists(&conn, "security_insights").unwrap());
        assert!(table_exists(&conn, "snapshot_diff_cache").unwrap());
        // Lockfile validation config table (v7)
        assert!(table_exists(&conn, "lockfile_validation_config").unwrap());
    }

    #[test]
    fn test_idempotent_migrations() {
        let conn = Connection::open_in_memory().unwrap();

        // Run migrations twice
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        // Should still be version 1
        let version = get_version(&conn).unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }
}
