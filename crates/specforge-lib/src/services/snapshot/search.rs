// Snapshot Search Service
// Provides searchable execution history with filtering and export

use serde::{Deserialize, Serialize};

use crate::models::snapshot::{
    ExecutionSnapshot, SnapshotDependency, SnapshotFilter, SnapshotStatus, TriggerSource,
};
use crate::repositories::SnapshotRepository;
use crate::utils::database::Database;

// =============================================================================
// Types
// =============================================================================

/// Search criteria for snapshots
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSearchCriteria {
    pub package_name: Option<String>,
    pub package_version: Option<String>,
    pub project_path: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub has_postinstall: Option<bool>,
    pub min_security_score: Option<i32>,
    pub max_security_score: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// A search result with snapshot and matched dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSearchResult {
    pub snapshot: ExecutionSnapshot,
    pub matched_dependencies: Vec<SnapshotDependency>,
    pub match_count: usize,
}

/// Search results summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultsSummary {
    pub total_snapshots: usize,
    pub total_matches: usize,
    pub date_range: Option<DateRange>,
    pub projects_involved: Vec<String>,
}

/// Date range for results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateRange {
    pub earliest: String,
    pub latest: String,
}

/// Full search response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub results: Vec<SnapshotSearchResult>,
    pub summary: SearchResultsSummary,
}

/// Timeline entry for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntry {
    pub snapshot_id: String,
    pub project_path: String,
    pub trigger_source: TriggerSource,
    pub created_at: String,
    pub status: SnapshotStatus,
    pub total_dependencies: i32,
    pub security_score: Option<i32>,
    pub postinstall_count: i32,
    pub has_security_issues: bool,
}

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Markdown,
    Html,
}

/// Security audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityAuditReport {
    pub generated_at: String,
    pub project_path: String,
    pub total_snapshots: usize,
    pub date_range: Option<DateRange>,
    pub risk_summary: RiskSummary,
    pub dependency_analysis: Vec<DependencyAnalysis>,
    pub security_events: Vec<SecurityEvent>,
}

/// Risk summary for audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskSummary {
    pub overall_risk: String,
    pub avg_security_score: Option<f64>,
    pub total_postinstall_scripts: i32,
    pub total_security_issues: i32,
}

/// Dependency analysis for audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyAnalysis {
    pub package_name: String,
    pub versions_seen: Vec<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub has_postinstall: bool,
    pub security_concerns: Vec<String>,
}

/// Security event for audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityEvent {
    pub timestamp: String,
    pub snapshot_id: String,
    pub event_type: String,
    pub description: String,
    pub severity: String,
}

// =============================================================================
// Service
// =============================================================================

/// Service for searching execution history
pub struct SnapshotSearchService {
    db: Database,
}

impl SnapshotSearchService {
    /// Create a new SnapshotSearchService
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Search snapshots by criteria
    pub fn search(&self, criteria: &SnapshotSearchCriteria) -> Result<SearchResponse, String> {
        let repo = SnapshotRepository::new(self.db.clone());

        // Build filter from criteria
        let filter = SnapshotFilter {
            project_path: criteria.project_path.clone(),
            from_date: criteria.from_date.clone(),
            to_date: criteria.to_date.clone(),
            limit: criteria.limit,
            offset: criteria.offset,
            ..Default::default()
        };

        let snapshots = repo.list_snapshots(&filter)?;
        let mut results = Vec::new();
        let mut total_matches = 0;
        let mut projects = std::collections::HashSet::new();

        for snapshot_item in snapshots {
            // Get full snapshot with dependencies
            let snapshot_with_deps = match repo.get_snapshot_with_dependencies(&snapshot_item.id)? {
                Some(s) => s,
                None => continue,
            };

            let snapshot = snapshot_with_deps.snapshot;
            let dependencies = snapshot_with_deps.dependencies;

            // Apply security score filter
            if let Some(min_score) = criteria.min_security_score {
                if snapshot.security_score.unwrap_or(0) < min_score {
                    continue;
                }
            }
            if let Some(max_score) = criteria.max_security_score {
                if snapshot.security_score.unwrap_or(100) > max_score {
                    continue;
                }
            }

            // Search dependencies if package_name is specified
            let matched_deps: Vec<SnapshotDependency> = if let Some(ref pkg_name) = criteria.package_name {
                dependencies
                    .into_iter()
                    .filter(|d| {
                        d.name.to_lowercase().contains(&pkg_name.to_lowercase())
                            || d.name.to_lowercase() == pkg_name.to_lowercase()
                    })
                    .filter(|d| {
                        if let Some(ref ver) = criteria.package_version {
                            d.version.contains(ver)
                        } else {
                            true
                        }
                    })
                    .filter(|d| {
                        if let Some(has_pi) = criteria.has_postinstall {
                            d.has_postinstall == has_pi
                        } else {
                            true
                        }
                    })
                    .collect()
            } else if criteria.has_postinstall.is_some() {
                // Filter by postinstall only
                dependencies
                    .into_iter()
                    .filter(|d| {
                        if let Some(has_pi) = criteria.has_postinstall {
                            d.has_postinstall == has_pi
                        } else {
                            true
                        }
                    })
                    .collect()
            } else {
                // No specific package search, include snapshot if other criteria match
                Vec::new()
            };

            // Only include if we have matches (when searching by package) or no package filter
            if criteria.package_name.is_some() && matched_deps.is_empty() {
                continue;
            }

            projects.insert(snapshot.project_path.clone());
            let match_count = matched_deps.len();
            total_matches += match_count;

            results.push(SnapshotSearchResult {
                snapshot,
                matched_dependencies: matched_deps,
                match_count,
            });
        }

        // Build summary
        let date_range = if !results.is_empty() {
            let earliest = results
                .iter()
                .map(|r| &r.snapshot.created_at)
                .min()
                .cloned();
            let latest = results
                .iter()
                .map(|r| &r.snapshot.created_at)
                .max()
                .cloned();

            match (earliest, latest) {
                (Some(e), Some(l)) => Some(DateRange {
                    earliest: e,
                    latest: l,
                }),
                _ => None,
            }
        } else {
            None
        };

        Ok(SearchResponse {
            summary: SearchResultsSummary {
                total_snapshots: results.len(),
                total_matches,
                date_range,
                projects_involved: projects.into_iter().collect(),
            },
            results,
        })
    }

    /// Get timeline of snapshots for a project
    pub fn get_timeline(
        &self,
        project_path: &str,
        limit: Option<i32>,
    ) -> Result<Vec<TimelineEntry>, String> {
        let repo = SnapshotRepository::new(self.db.clone());

        let filter = SnapshotFilter {
            project_path: Some(project_path.to_string()),
            limit,
            ..Default::default()
        };

        let snapshots = repo.list_snapshots(&filter)?;
        let mut timeline = Vec::new();

        for snapshot_item in snapshots {
            // Get insights to check for security issues
            let insights = repo.list_insights(&snapshot_item.id).unwrap_or_default();
            let has_security_issues = insights
                .iter()
                .any(|i| i.severity >= crate::models::security_insight::InsightSeverity::Medium);

            timeline.push(TimelineEntry {
                snapshot_id: snapshot_item.id,
                project_path: snapshot_item.project_path,
                trigger_source: snapshot_item.trigger_source,
                created_at: snapshot_item.created_at,
                status: snapshot_item.status,
                total_dependencies: snapshot_item.total_dependencies,
                security_score: snapshot_item.security_score,
                postinstall_count: snapshot_item.postinstall_count,
                has_security_issues,
            });
        }

        Ok(timeline)
    }

    /// Generate security audit report
    pub fn generate_audit_report(&self, project_path: &str) -> Result<SecurityAuditReport, String> {
        let repo = SnapshotRepository::new(self.db.clone());

        let filter = SnapshotFilter {
            project_path: Some(project_path.to_string()),
            ..Default::default()
        };

        let snapshots = repo.list_snapshots(&filter)?;

        if snapshots.is_empty() {
            return Err("No snapshots found for this project".to_string());
        }

        // Collect all dependencies and insights
        let mut all_deps: std::collections::HashMap<String, Vec<(String, String, bool)>> =
            std::collections::HashMap::new();
        let mut security_events = Vec::new();
        let mut total_postinstall = 0;
        let mut total_security_issues = 0;
        let mut security_scores = Vec::new();

        for snapshot_item in &snapshots {
            if let Some(snapshot_with_deps) =
                repo.get_snapshot_with_dependencies(&snapshot_item.id)?
            {
                let snapshot = &snapshot_with_deps.snapshot;

                if let Some(score) = snapshot.security_score {
                    security_scores.push(score);
                }

                for dep in &snapshot_with_deps.dependencies {
                    all_deps
                        .entry(dep.name.clone())
                        .or_default()
                        .push((dep.version.clone(), snapshot.created_at.clone(), dep.has_postinstall));

                    if dep.has_postinstall {
                        total_postinstall += 1;
                    }
                }
            }

            // Get security insights as events
            let insights = repo.list_insights(&snapshot_item.id).unwrap_or_default();
            for insight in insights {
                if insight.severity >= crate::models::security_insight::InsightSeverity::Medium {
                    total_security_issues += 1;
                }

                security_events.push(SecurityEvent {
                    timestamp: insight.created_at,
                    snapshot_id: snapshot_item.id.clone(),
                    event_type: format!("{:?}", insight.insight_type),
                    description: insight.title,
                    severity: insight.severity.as_str().to_string(),
                });
            }
        }

        // Build dependency analysis
        let mut dependency_analysis: Vec<DependencyAnalysis> = all_deps
            .into_iter()
            .map(|(name, versions)| {
                let mut unique_versions: Vec<String> = versions
                    .iter()
                    .map(|(v, _, _)| v.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                unique_versions.sort();

                let has_postinstall = versions.iter().any(|(_, _, pi)| *pi);
                let dates: Vec<&String> = versions.iter().map(|(_, d, _)| d).collect();
                let first_seen = dates.iter().min().map(|s| (*s).clone()).unwrap_or_default();
                let last_seen = dates.iter().max().map(|s| (*s).clone()).unwrap_or_default();

                DependencyAnalysis {
                    package_name: name,
                    versions_seen: unique_versions,
                    first_seen,
                    last_seen,
                    has_postinstall,
                    security_concerns: Vec::new(),
                }
            })
            .collect();

        dependency_analysis.sort_by(|a, b| a.package_name.cmp(&b.package_name));

        // Calculate averages and risk
        let avg_security_score = if !security_scores.is_empty() {
            Some(security_scores.iter().sum::<i32>() as f64 / security_scores.len() as f64)
        } else {
            None
        };

        let overall_risk = if total_security_issues > 10 {
            "Critical"
        } else if total_security_issues > 5 {
            "High"
        } else if total_security_issues > 0 {
            "Medium"
        } else {
            "Low"
        };

        // Date range
        let date_range = if !snapshots.is_empty() {
            Some(DateRange {
                earliest: snapshots.last().map(|s| s.created_at.clone()).unwrap_or_default(),
                latest: snapshots.first().map(|s| s.created_at.clone()).unwrap_or_default(),
            })
        } else {
            None
        };

        // Sort security events by timestamp descending
        security_events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(SecurityAuditReport {
            generated_at: chrono::Utc::now().to_rfc3339(),
            project_path: project_path.to_string(),
            total_snapshots: snapshots.len(),
            date_range,
            risk_summary: RiskSummary {
                overall_risk: overall_risk.to_string(),
                avg_security_score,
                total_postinstall_scripts: total_postinstall,
                total_security_issues,
            },
            dependency_analysis,
            security_events,
        })
    }

    /// Export audit report in specified format
    pub fn export_report(&self, report: &SecurityAuditReport, format: ExportFormat) -> String {
        match format {
            ExportFormat::Json => serde_json::to_string_pretty(report).unwrap_or_default(),
            ExportFormat::Markdown => self.report_to_markdown(report),
            ExportFormat::Html => self.report_to_html(report),
        }
    }

    fn report_to_markdown(&self, report: &SecurityAuditReport) -> String {
        let mut md = String::new();

        md.push_str("# Security Audit Report\n\n");
        md.push_str(&format!("**Generated:** {}\n\n", report.generated_at));
        md.push_str(&format!("**Project:** {}\n\n", report.project_path));

        if let Some(ref range) = report.date_range {
            md.push_str(&format!(
                "**Date Range:** {} to {}\n\n",
                range.earliest, range.latest
            ));
        }

        md.push_str("## Risk Summary\n\n");
        md.push_str(&format!(
            "- **Overall Risk:** {}\n",
            report.risk_summary.overall_risk
        ));
        if let Some(avg) = report.risk_summary.avg_security_score {
            md.push_str(&format!("- **Average Security Score:** {:.1}\n", avg));
        }
        md.push_str(&format!(
            "- **Total Postinstall Scripts:** {}\n",
            report.risk_summary.total_postinstall_scripts
        ));
        md.push_str(&format!(
            "- **Security Issues:** {}\n\n",
            report.risk_summary.total_security_issues
        ));

        md.push_str("## Dependencies\n\n");
        md.push_str("| Package | Versions | Has Postinstall |\n");
        md.push_str("|---------|----------|----------------|\n");
        for dep in &report.dependency_analysis {
            md.push_str(&format!(
                "| {} | {} | {} |\n",
                dep.package_name,
                dep.versions_seen.join(", "),
                if dep.has_postinstall { "Yes" } else { "No" }
            ));
        }

        if !report.security_events.is_empty() {
            md.push_str("\n## Security Events\n\n");
            for event in &report.security_events {
                md.push_str(&format!(
                    "- **[{}]** {} - {}\n",
                    event.severity, event.event_type, event.description
                ));
            }
        }

        md
    }

    fn report_to_html(&self, report: &SecurityAuditReport) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str("<title>Security Audit Report</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 900px; margin: 0 auto; padding: 20px; background: #1a1a1a; color: #e0e0e0; }\n");
        html.push_str("h1, h2 { color: #00d4ff; }\n");
        html.push_str("table { border-collapse: collapse; width: 100%; margin: 20px 0; }\n");
        html.push_str("th, td { border: 1px solid #333; padding: 8px; text-align: left; }\n");
        html.push_str("th { background: #2a2a2a; }\n");
        html.push_str(".risk-low { color: #4ade80; }\n");
        html.push_str(".risk-medium { color: #facc15; }\n");
        html.push_str(".risk-high { color: #f97316; }\n");
        html.push_str(".risk-critical { color: #ef4444; }\n");
        html.push_str("</style>\n</head>\n<body>\n");

        html.push_str("<h1>Security Audit Report</h1>\n");
        html.push_str(&format!("<p><strong>Generated:</strong> {}</p>\n", report.generated_at));
        html.push_str(&format!("<p><strong>Project:</strong> {}</p>\n", report.project_path));

        let risk_class = match report.risk_summary.overall_risk.as_str() {
            "Low" => "risk-low",
            "Medium" => "risk-medium",
            "High" => "risk-high",
            _ => "risk-critical",
        };

        html.push_str("<h2>Risk Summary</h2>\n<ul>\n");
        html.push_str(&format!(
            "<li><strong>Overall Risk:</strong> <span class=\"{}\">{}</span></li>\n",
            risk_class, report.risk_summary.overall_risk
        ));
        if let Some(avg) = report.risk_summary.avg_security_score {
            html.push_str(&format!(
                "<li><strong>Average Security Score:</strong> {:.1}</li>\n",
                avg
            ));
        }
        html.push_str(&format!(
            "<li><strong>Postinstall Scripts:</strong> {}</li>\n",
            report.risk_summary.total_postinstall_scripts
        ));
        html.push_str(&format!(
            "<li><strong>Security Issues:</strong> {}</li>\n",
            report.risk_summary.total_security_issues
        ));
        html.push_str("</ul>\n");

        html.push_str("<h2>Dependencies</h2>\n<table>\n");
        html.push_str("<tr><th>Package</th><th>Versions</th><th>Postinstall</th></tr>\n");
        for dep in &report.dependency_analysis {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                dep.package_name,
                dep.versions_seen.join(", "),
                if dep.has_postinstall { "Yes" } else { "No" }
            ));
        }
        html.push_str("</table>\n");

        html.push_str("</body>\n</html>");

        html
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_serialization() {
        let format = ExportFormat::Json;
        let json = serde_json::to_string(&format).unwrap();
        assert_eq!(json, "\"json\"");
    }
}
