//! Built-in step templates for workflows
//!
//! Contains predefined templates for common workflow steps.

use super::types::StepTemplateInfo;

/// Get built-in step templates (subset of most commonly used)
pub fn get_builtin_templates() -> Vec<StepTemplateInfo> {
    vec![
        // Package Manager
        StepTemplateInfo {
            id: "pm-install".to_string(),
            name: "Install Dependencies".to_string(),
            command: "{pm} install".to_string(),
            category: "package-manager".to_string(),
            description: Some("Install project dependencies".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "pm-build".to_string(),
            name: "Build Project".to_string(),
            command: "{pm} run build".to_string(),
            category: "package-manager".to_string(),
            description: Some("Run the build script".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "pm-test".to_string(),
            name: "Run Tests".to_string(),
            command: "{pm} test".to_string(),
            category: "package-manager".to_string(),
            description: Some("Run test suite".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "pm-dev".to_string(),
            name: "Start Dev Server".to_string(),
            command: "{pm} run dev".to_string(),
            category: "package-manager".to_string(),
            description: Some("Start development server".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "pm-clean-install".to_string(),
            name: "Clean Install".to_string(),
            command: "rm -rf node_modules && {pm} install".to_string(),
            category: "package-manager".to_string(),
            description: Some("Remove node_modules and reinstall".to_string()),
            is_custom: false,
        },
        // Git
        StepTemplateInfo {
            id: "git-status".to_string(),
            name: "Git Status".to_string(),
            command: "git status".to_string(),
            category: "git".to_string(),
            description: Some("Show working tree status".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "git-add-all".to_string(),
            name: "Git Add All".to_string(),
            command: "git add .".to_string(),
            category: "git".to_string(),
            description: Some("Stage all changes".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "git-commit".to_string(),
            name: "Git Commit".to_string(),
            command: "git commit -m \"Update\"".to_string(),
            category: "git".to_string(),
            description: Some("Commit staged changes".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "git-push".to_string(),
            name: "Git Push".to_string(),
            command: "git push".to_string(),
            category: "git".to_string(),
            description: Some("Push to remote".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "git-pull".to_string(),
            name: "Git Pull".to_string(),
            command: "git pull".to_string(),
            category: "git".to_string(),
            description: Some("Pull from remote".to_string()),
            is_custom: false,
        },
        // Docker
        StepTemplateInfo {
            id: "docker-build".to_string(),
            name: "Docker Build".to_string(),
            command: "docker build -t myapp .".to_string(),
            category: "docker".to_string(),
            description: Some("Build Docker image".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "docker-compose-up".to_string(),
            name: "Docker Compose Up".to_string(),
            command: "docker-compose up -d".to_string(),
            category: "docker".to_string(),
            description: Some("Start services in detached mode".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "docker-compose-down".to_string(),
            name: "Docker Compose Down".to_string(),
            command: "docker-compose down".to_string(),
            category: "docker".to_string(),
            description: Some("Stop and remove containers".to_string()),
            is_custom: false,
        },
        // Testing
        StepTemplateInfo {
            id: "test-coverage".to_string(),
            name: "Test with Coverage".to_string(),
            command: "{pm} run test:coverage".to_string(),
            category: "testing".to_string(),
            description: Some("Run tests with coverage report".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "test-watch".to_string(),
            name: "Test Watch Mode".to_string(),
            command: "{pm} run test:watch".to_string(),
            category: "testing".to_string(),
            description: Some("Run tests in watch mode".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "vitest".to_string(),
            name: "Vitest".to_string(),
            command: "vitest".to_string(),
            category: "testing".to_string(),
            description: Some("Run Vitest test runner".to_string()),
            is_custom: false,
        },
        // Code Quality
        StepTemplateInfo {
            id: "lint".to_string(),
            name: "Lint".to_string(),
            command: "{pm} run lint".to_string(),
            category: "code-quality".to_string(),
            description: Some("Run linter".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "lint-fix".to_string(),
            name: "Lint Fix".to_string(),
            command: "{pm} run lint -- --fix".to_string(),
            category: "code-quality".to_string(),
            description: Some("Run linter with auto-fix".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "format".to_string(),
            name: "Format".to_string(),
            command: "{pm} run format".to_string(),
            category: "code-quality".to_string(),
            description: Some("Format code".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "typecheck".to_string(),
            name: "Type Check".to_string(),
            command: "tsc --noEmit".to_string(),
            category: "code-quality".to_string(),
            description: Some("Run TypeScript type checking".to_string()),
            is_custom: false,
        },
        // Shell
        StepTemplateInfo {
            id: "shell-echo".to_string(),
            name: "Echo".to_string(),
            command: "echo \"Hello World\"".to_string(),
            category: "shell".to_string(),
            description: Some("Print message to stdout".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "shell-sleep".to_string(),
            name: "Sleep".to_string(),
            command: "sleep 5".to_string(),
            category: "shell".to_string(),
            description: Some("Wait for 5 seconds".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "shell-env".to_string(),
            name: "Print Environment".to_string(),
            command: "env".to_string(),
            category: "shell".to_string(),
            description: Some("Print environment variables".to_string()),
            is_custom: false,
        },
        // Rust/Cargo
        StepTemplateInfo {
            id: "cargo-build".to_string(),
            name: "Cargo Build".to_string(),
            command: "cargo build".to_string(),
            category: "rust".to_string(),
            description: Some("Build Rust project".to_string()),
            is_custom: false,
        },
        StepTemplateInfo {
            id: "cargo-test".to_string(),
            name: "Cargo Test".to_string(),
            command: "cargo test".to_string(),
            category: "rust".to_string(),
            description: Some("Run Rust tests".to_string()),
            is_custom: false,
        },
    ]
}
