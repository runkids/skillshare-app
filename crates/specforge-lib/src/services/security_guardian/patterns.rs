// Security Guardian - Pattern-based Analysis
// Offline security analysis using pattern matching without cloud AI

use serde::{Deserialize, Serialize};
use strsim::levenshtein;

use crate::models::snapshot::DependencyChange;

// =============================================================================
// Types
// =============================================================================

/// Security alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// A security pattern match result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternAlert {
    pub alert_type: PatternAlertType,
    pub severity: AlertSeverity,
    pub package_name: String,
    pub title: String,
    pub description: String,
    pub recommendation: Option<String>,
}

/// Types of pattern-based alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternAlertType {
    Typosquatting,
    SuspiciousVersion,
    MajorVersionJump,
    NewPostinstall,
    PostinstallChanged,
    UnexpectedDowngrade,
    SuspiciousPackageName,
    DeprecatedPackage,
}

/// Result of pattern-based analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternAnalysisResult {
    pub alerts: Vec<PatternAlert>,
    pub summary: PatternAnalysisSummary,
}

/// Summary of pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternAnalysisSummary {
    pub total_alerts: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub info_count: usize,
    pub risk_score: u8, // 0-100
}

// =============================================================================
// Top NPM Packages (for typosquatting detection)
// =============================================================================

/// Top 500 most downloaded npm packages
/// Updated quarterly with app releases
/// Source: npm registry download statistics
pub static POPULAR_PACKAGES: &[&str] = &[
    // Core/utilities
    "lodash", "underscore", "ramda", "date-fns", "moment", "dayjs", "luxon",
    "uuid", "nanoid", "crypto-js", "bcrypt", "argon2",
    // React ecosystem
    "react", "react-dom", "react-router", "react-router-dom", "redux",
    "react-redux", "@reduxjs/toolkit", "mobx", "mobx-react", "zustand",
    "jotai", "recoil", "valtio", "immer", "use-immer",
    "next", "gatsby", "remix", "@remix-run/react", "@remix-run/node",
    "styled-components", "@emotion/react", "@emotion/styled",
    "tailwindcss", "@tailwindcss/forms", "@tailwindcss/typography",
    "framer-motion", "react-spring", "@react-spring/web",
    "react-query", "@tanstack/react-query", "swr", "axios", "ky",
    "react-hook-form", "formik", "yup", "zod", "@hookform/resolvers",
    "react-select", "react-datepicker", "react-table", "@tanstack/react-table",
    "react-icons", "lucide-react", "@heroicons/react", "@radix-ui/react-icons",
    // Vue ecosystem
    "vue", "vue-router", "vuex", "pinia", "@vue/compiler-sfc",
    "nuxt", "@nuxt/kit", "vitepress", "vuepress",
    // Angular
    "@angular/core", "@angular/common", "@angular/router", "@angular/forms",
    "@angular/platform-browser", "@angular/cli", "rxjs", "zone.js",
    // Build tools
    "webpack", "webpack-cli", "webpack-dev-server", "vite", "esbuild",
    "rollup", "parcel", "turbo", "nx", "lerna", "rush",
    "babel-core", "@babel/core", "@babel/preset-env", "@babel/preset-react",
    "@babel/preset-typescript", "typescript", "ts-node", "tsx",
    // Testing
    "jest", "@jest/core", "mocha", "chai", "sinon", "jasmine",
    "vitest", "@vitest/ui", "@vitest/coverage-v8",
    "cypress", "@cypress/react", "playwright", "@playwright/test",
    "testing-library", "@testing-library/react", "@testing-library/jest-dom",
    "@testing-library/user-event", "@testing-library/dom",
    // Linting/formatting
    "eslint", "prettier", "stylelint", "husky", "lint-staged",
    "@typescript-eslint/parser", "@typescript-eslint/eslint-plugin",
    "eslint-plugin-react", "eslint-plugin-react-hooks",
    "eslint-config-prettier", "eslint-plugin-prettier",
    // Node.js frameworks
    "express", "koa", "fastify", "hapi", "nest", "@nestjs/core",
    "restify", "connect", "body-parser", "cors", "helmet",
    "compression", "morgan", "winston", "pino", "bunyan",
    "passport", "passport-local", "passport-jwt", "jsonwebtoken", "jose",
    // Database
    "mongoose", "sequelize", "typeorm", "prisma", "@prisma/client",
    "knex", "pg", "mysql", "mysql2", "sqlite3", "better-sqlite3",
    "redis", "ioredis", "mongodb", "dynamodb",
    // HTTP/networking
    "axios", "node-fetch", "got", "superagent", "request",
    "socket.io", "socket.io-client", "ws", "websocket",
    "http-proxy", "http-proxy-middleware",
    // File handling
    "fs-extra", "glob", "globby", "fast-glob", "chokidar",
    "multer", "formidable", "busboy", "sharp", "jimp",
    "archiver", "unzipper", "tar", "adm-zip",
    // CLI tools
    "commander", "yargs", "meow", "inquirer", "prompts",
    "chalk", "colors", "ora", "cli-progress", "figlet",
    "execa", "shelljs", "cross-spawn", "npm-run-all",
    // Validation
    "joi", "yup", "zod", "ajv", "validator", "class-validator",
    // Markdown/docs
    "marked", "markdown-it", "remark", "rehype", "unified",
    "gray-matter", "front-matter",
    // Email
    "nodemailer", "@sendgrid/mail", "mailgun-js", "postmark",
    // Cloud/AWS
    "aws-sdk", "@aws-sdk/client-s3", "@aws-sdk/client-dynamodb",
    "@google-cloud/storage", "@azure/storage-blob",
    "firebase", "firebase-admin", "@firebase/app",
    // GraphQL
    "graphql", "apollo-server", "@apollo/server", "@apollo/client",
    "graphql-yoga", "type-graphql", "graphql-tools",
    // Misc popular
    "dotenv", "cross-env", "env-cmd", "config",
    "debug", "semver", "minimatch", "micromatch",
    "lodash.debounce", "lodash.throttle", "lodash.clonedeep",
    "classnames", "clsx", "cva", "class-variance-authority",
    "prop-types", "invariant", "warning",
    // Security-related
    "helmet", "csurf", "xss", "sanitize-html", "dompurify",
    "rate-limiter-flexible", "express-rate-limit",
    // Monorepo
    "lerna", "nx", "turbo", "pnpm", "yarn",
    // Dev dependencies
    "@types/node", "@types/react", "@types/react-dom",
    "@types/lodash", "@types/express", "@types/jest",
    // Popular UI libraries
    "@mui/material", "@mui/icons-material", "@chakra-ui/react",
    "antd", "ant-design-vue", "element-plus", "element-ui",
    "@headlessui/react", "@radix-ui/react-dialog", "@radix-ui/react-dropdown-menu",
    "react-bootstrap", "reactstrap", "bootstrap",
    // Animation
    "gsap", "anime", "animejs", "lottie-web", "lottie-react",
    "motion", "auto-animate", "@formkit/auto-animate",
    // Charts
    "chart.js", "react-chartjs-2", "recharts", "victory",
    "d3", "d3-scale", "d3-shape", "@visx/visx",
    "echarts", "echarts-for-react", "apexcharts", "react-apexcharts",
    // Maps
    "leaflet", "react-leaflet", "mapbox-gl", "react-map-gl",
    "@react-google-maps/api", "google-maps-react",
    // State machines
    "xstate", "@xstate/react", "robot3",
    // i18n
    "i18next", "react-i18next", "react-intl", "vue-i18n",
    // PDF
    "pdfkit", "pdf-lib", "jspdf", "react-pdf", "@react-pdf/renderer",
    // Date/time
    "date-fns", "dayjs", "moment", "luxon", "chrono-node",
    // Rich text
    "slate", "slate-react", "draft-js", "quill", "react-quill",
    "prosemirror", "tiptap", "@tiptap/react", "lexical",
    // Code editors
    "monaco-editor", "@monaco-editor/react", "codemirror",
    "@codemirror/state", "@codemirror/view",
    "prismjs", "highlight.js", "shiki",
    // Virtual lists
    "react-window", "react-virtualized", "@tanstack/react-virtual",
    // DnD
    "react-dnd", "react-dnd-html5-backend", "@dnd-kit/core",
    "react-beautiful-dnd", "react-sortable-hoc",
    // Payments
    "stripe", "@stripe/stripe-js", "@stripe/react-stripe-js",
    "paypal-rest-sdk", "@paypal/react-paypal-js",
    // SSR/SSG
    "next", "nuxt", "gatsby", "astro", "remix",
    "sveltekit", "@sveltejs/kit", "vitepress",
    // Desktop
    "electron", "tauri", "@tauri-apps/api", "neutralino",
    // Mobile
    "react-native", "expo", "@expo/vector-icons",
    "nativescript", "@nativescript/core",
    // WebSockets
    "socket.io", "socket.io-client", "ws", "websocket",
    "pusher", "pusher-js", "ably",
    // Task runners
    "gulp", "grunt", "npm-run-all", "concurrently", "wait-on",
];

// =============================================================================
// Typosquatting Detection
// =============================================================================

/// Result of typosquatting check
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TyposquattingResult {
    pub is_suspicious: bool,
    pub similar_to: Option<String>,
    pub edit_distance: usize,
    pub confidence: f32, // 0.0-1.0
}

/// Check if a package name is potentially typosquatting a popular package
pub fn check_typosquatting(package_name: &str, threshold: usize) -> TyposquattingResult {
    let name_lower = package_name.to_lowercase();

    // Skip scoped packages for now (e.g., @types/node)
    if name_lower.starts_with('@') {
        return TyposquattingResult {
            is_suspicious: false,
            similar_to: None,
            edit_distance: 0,
            confidence: 0.0,
        };
    }

    let mut best_match: Option<(&str, usize)> = None;

    for &popular in POPULAR_PACKAGES {
        let popular_lower = popular.to_lowercase();

        // Skip exact matches
        if name_lower == popular_lower {
            return TyposquattingResult {
                is_suspicious: false,
                similar_to: None,
                edit_distance: 0,
                confidence: 0.0,
            };
        }

        // Skip if lengths differ too much (optimization)
        let len_diff = (name_lower.len() as i32 - popular_lower.len() as i32).abs() as usize;
        if len_diff > threshold {
            continue;
        }

        let distance = levenshtein(&name_lower, &popular_lower);

        if distance <= threshold && distance > 0 {
            if best_match.is_none() || distance < best_match.unwrap().1 {
                best_match = Some((popular, distance));
            }
        }
    }

    match best_match {
        Some((similar, distance)) => {
            // Calculate confidence based on distance and string length
            let max_len = similar.len().max(package_name.len()) as f32;
            let confidence = 1.0 - (distance as f32 / max_len);

            TyposquattingResult {
                is_suspicious: true,
                similar_to: Some(similar.to_string()),
                edit_distance: distance,
                confidence,
            }
        }
        None => TyposquattingResult {
            is_suspicious: false,
            similar_to: None,
            edit_distance: 0,
            confidence: 0.0,
        },
    }
}

/// Check multiple packages for typosquatting
pub fn check_packages_typosquatting(
    packages: &[String],
    threshold: usize,
) -> Vec<(String, TyposquattingResult)> {
    packages
        .iter()
        .map(|pkg| {
            let result = check_typosquatting(pkg, threshold);
            (pkg.clone(), result)
        })
        .filter(|(_, result)| result.is_suspicious)
        .collect()
}

// =============================================================================
// Pattern-based Security Analysis
// =============================================================================

/// Analyze dependency changes for security patterns (offline mode)
pub fn analyze_dependency_changes(changes: &[DependencyChange]) -> PatternAnalysisResult {
    let mut alerts = Vec::new();

    for change in changes {
        // Check for typosquatting on new packages
        if change.change_type == crate::models::snapshot::DependencyChangeType::Added {
            let typo_result = check_typosquatting(&change.name, 2);
            if typo_result.is_suspicious {
                alerts.push(PatternAlert {
                    alert_type: PatternAlertType::Typosquatting,
                    severity: AlertSeverity::High,
                    package_name: change.name.clone(),
                    title: format!("Potential typosquatting: {}", change.name),
                    description: format!(
                        "Package '{}' is similar to popular package '{}' (edit distance: {}). \
                         This could be a typosquatting attack.",
                        change.name,
                        typo_result.similar_to.as_deref().unwrap_or("unknown"),
                        typo_result.edit_distance
                    ),
                    recommendation: Some(format!(
                        "Verify you intended to install '{}' and not '{}'",
                        change.name,
                        typo_result.similar_to.as_deref().unwrap_or("the similar package")
                    )),
                });
            }

            // Check for suspicious package names
            if is_suspicious_package_name(&change.name) {
                alerts.push(PatternAlert {
                    alert_type: PatternAlertType::SuspiciousPackageName,
                    severity: AlertSeverity::Medium,
                    package_name: change.name.clone(),
                    title: format!("Suspicious package name: {}", change.name),
                    description: "Package name contains patterns often seen in malicious packages".to_string(),
                    recommendation: Some("Review the package source and maintainer before using".to_string()),
                });
            }
        }

        // Check for postinstall script changes
        if change.postinstall_changed {
            let severity = if change.change_type == crate::models::snapshot::DependencyChangeType::Added {
                AlertSeverity::High
            } else {
                AlertSeverity::Medium
            };

            let alert_type = if change.new_postinstall.is_some() && change.old_postinstall.is_none() {
                PatternAlertType::NewPostinstall
            } else {
                PatternAlertType::PostinstallChanged
            };

            alerts.push(PatternAlert {
                alert_type,
                severity,
                package_name: change.name.clone(),
                title: format!("Postinstall script change in {}", change.name),
                description: match (&change.old_postinstall, &change.new_postinstall) {
                    (None, Some(new)) => format!("New postinstall script added: {}", truncate_script(new, 100)),
                    (Some(_), None) => "Postinstall script was removed".to_string(),
                    (Some(_), Some(new)) => format!("Postinstall script changed to: {}", truncate_script(new, 100)),
                    (None, None) => "Postinstall script status changed".to_string(),
                },
                recommendation: Some("Review the postinstall script content for suspicious commands".to_string()),
            });
        }

        // Check for major version jumps
        if let (Some(old_ver), Some(new_ver)) = (&change.old_version, &change.new_version) {
            if let Some(alert) = check_version_jump(old_ver, new_ver, &change.name) {
                alerts.push(alert);
            }
        }

        // Check for unexpected downgrades
        if change.change_type == crate::models::snapshot::DependencyChangeType::Updated {
            if let (Some(old_ver), Some(new_ver)) = (&change.old_version, &change.new_version) {
                if is_downgrade(old_ver, new_ver) {
                    alerts.push(PatternAlert {
                        alert_type: PatternAlertType::UnexpectedDowngrade,
                        severity: AlertSeverity::Medium,
                        package_name: change.name.clone(),
                        title: format!("Unexpected version downgrade: {}", change.name),
                        description: format!(
                            "Package was downgraded from {} to {}. This is unusual and may indicate \
                             a lockfile manipulation or rollback attack.",
                            old_ver, new_ver
                        ),
                        recommendation: Some("Verify this downgrade was intentional".to_string()),
                    });
                }
            }
        }
    }

    // Calculate summary
    let summary = calculate_summary(&alerts);

    PatternAnalysisResult { alerts, summary }
}

/// Check if a package name has suspicious patterns
fn is_suspicious_package_name(name: &str) -> bool {
    let suspicious_patterns = [
        // Common typosquatting suffixes/prefixes
        "-js-",
        "-node-",
        "node-",
        "-npm",
        "npm-",
        // Suspicious keywords
        "malware",
        "hack",
        "crack",
        "keygen",
        "loader",
        "injector",
        // Copy patterns
        "-copy",
        "-clone",
        "-fork",
        "copy-of-",
        // Test/temp patterns in production deps
        "test-pkg",
        "temp-pkg",
        "my-test",
    ];

    let name_lower = name.to_lowercase();

    for pattern in &suspicious_patterns {
        if name_lower.contains(pattern) {
            return true;
        }
    }

    // Check for excessive hyphens (often typosquatting)
    if name.matches('-').count() > 4 {
        return true;
    }

    // Check for mixed separators (lodash_debounce vs lodash-debounce)
    if name.contains('-') && name.contains('_') {
        return true;
    }

    false
}

/// Check for suspicious version jumps
fn check_version_jump(old_version: &str, new_version: &str, package_name: &str) -> Option<PatternAlert> {
    let old_parts: Vec<u32> = old_version
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let new_parts: Vec<u32> = new_version
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    if old_parts.is_empty() || new_parts.is_empty() {
        return None;
    }

    // Check for major version jump > 2
    if new_parts[0] > old_parts[0] + 2 {
        return Some(PatternAlert {
            alert_type: PatternAlertType::MajorVersionJump,
            severity: AlertSeverity::Medium,
            package_name: package_name.to_string(),
            title: format!("Large major version jump: {}", package_name),
            description: format!(
                "Package jumped from v{} to v{}. Large version jumps may indicate \
                 a package hijacking or unintended upgrade.",
                old_version, new_version
            ),
            recommendation: Some("Review the changelog for breaking changes".to_string()),
        });
    }

    // Check for suspicious version patterns (0.0.x to high versions)
    if old_parts[0] == 0 && old_parts.get(1) == Some(&0) && new_parts[0] > 1 {
        return Some(PatternAlert {
            alert_type: PatternAlertType::SuspiciousVersion,
            severity: AlertSeverity::High,
            package_name: package_name.to_string(),
            title: format!("Suspicious version change: {}", package_name),
            description: format!(
                "Package jumped from development version {} to {}. \
                 This pattern is sometimes seen in package hijacking.",
                old_version, new_version
            ),
            recommendation: Some("Verify the package maintainer hasn't changed".to_string()),
        });
    }

    None
}

/// Check if new version is lower than old version (downgrade)
fn is_downgrade(old_version: &str, new_version: &str) -> bool {
    let old_parts: Vec<u32> = old_version
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let new_parts: Vec<u32> = new_version
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    if old_parts.is_empty() || new_parts.is_empty() {
        return false;
    }

    // Compare version tuples
    for i in 0..old_parts.len().min(new_parts.len()) {
        if new_parts[i] < old_parts[i] {
            return true;
        } else if new_parts[i] > old_parts[i] {
            return false;
        }
    }

    // If all compared parts are equal, check length
    new_parts.len() < old_parts.len()
}

/// Truncate script for display
fn truncate_script(script: &str, max_len: usize) -> String {
    if script.len() <= max_len {
        script.to_string()
    } else {
        format!("{}...", &script[..max_len])
    }
}

/// Calculate summary statistics
fn calculate_summary(alerts: &[PatternAlert]) -> PatternAnalysisSummary {
    let mut critical = 0;
    let mut high = 0;
    let mut medium = 0;
    let mut low = 0;
    let mut info = 0;

    for alert in alerts {
        match alert.severity {
            AlertSeverity::Critical => critical += 1,
            AlertSeverity::High => high += 1,
            AlertSeverity::Medium => medium += 1,
            AlertSeverity::Low => low += 1,
            AlertSeverity::Info => info += 1,
        }
    }

    // Calculate risk score (0-100)
    let risk_score = (critical * 25 + high * 15 + medium * 8 + low * 3 + info * 1)
        .min(100) as u8;

    PatternAnalysisSummary {
        total_alerts: alerts.len(),
        critical_count: critical,
        high_count: high,
        medium_count: medium,
        low_count: low,
        info_count: info,
        risk_score,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typosquatting_exact_match() {
        let result = check_typosquatting("lodash", 2);
        assert!(!result.is_suspicious);
    }

    #[test]
    fn test_typosquatting_detected() {
        let result = check_typosquatting("loadsh", 2);
        assert!(result.is_suspicious);
        assert_eq!(result.similar_to, Some("lodash".to_string()));
        assert_eq!(result.edit_distance, 2);
    }

    #[test]
    fn test_typosquatting_not_detected() {
        let result = check_typosquatting("my-custom-package", 2);
        assert!(!result.is_suspicious);
    }

    #[test]
    fn test_suspicious_package_name() {
        assert!(is_suspicious_package_name("lodash-copy"));
        assert!(is_suspicious_package_name("react-hack-utils"));
        assert!(!is_suspicious_package_name("react-router-dom"));
    }

    #[test]
    fn test_version_downgrade() {
        assert!(is_downgrade("2.0.0", "1.0.0"));
        assert!(is_downgrade("1.5.0", "1.4.0"));
        assert!(!is_downgrade("1.0.0", "2.0.0"));
        assert!(!is_downgrade("1.0.0", "1.0.0"));
    }

    #[test]
    fn test_version_jump_detection() {
        let alert = check_version_jump("1.0.0", "5.0.0", "test-pkg");
        assert!(alert.is_some());

        let alert = check_version_jump("1.0.0", "2.0.0", "test-pkg");
        assert!(alert.is_none());
    }
}
