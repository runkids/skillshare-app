// Security Guardian - Insights Service
// Aggregates security insights from snapshot history and calculates risk scores

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::models::security_insight::{
    DependencyHealth, FrequentUpdater, HealthFactor, InsightSeverity, InsightSummary,
    InsightType, SecurityInsight,
};
use crate::models::snapshot::SnapshotFilter;
use crate::repositories::SnapshotRepository;
use crate::utils::database::Database;

// =============================================================================
// Types
// =============================================================================

/// Project security overview
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSecurityOverview {
    pub project_path: String,
    pub risk_score: i32,  // 0-100
    pub risk_level: OverallRiskLevel,
    pub total_snapshots: usize,
    pub latest_snapshot_id: Option<String>,
    pub latest_snapshot_date: Option<String>,
    pub insight_summary: InsightSummary,
    pub typosquatting_alerts: Vec<TyposquattingAlertInfo>,
    pub frequent_updaters: Vec<FrequentUpdater>,
    pub dependency_health: Vec<DependencyHealth>,
}

/// Risk level derived from risk score
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverallRiskLevel {
    Low,      // 0-25
    Medium,   // 26-50
    High,     // 51-75
    Critical, // 76-100
}

impl From<i32> for OverallRiskLevel {
    fn from(score: i32) -> Self {
        match score {
            0..=25 => OverallRiskLevel::Low,
            26..=50 => OverallRiskLevel::Medium,
            51..=75 => OverallRiskLevel::High,
            _ => OverallRiskLevel::Critical,
        }
    }
}

/// Typosquatting alert info for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TyposquattingAlertInfo {
    pub package_name: String,
    pub similar_to: String,
    pub first_seen: String,
    pub snapshot_id: String,
}

// =============================================================================
// Service
// =============================================================================

/// Service for security insights and risk analysis
pub struct SecurityInsightsService {
    db: Database,
}

impl SecurityInsightsService {
    /// Create a new SecurityInsightsService
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Get project security overview with risk score
    pub fn get_project_overview(&self, project_path: &str) -> Result<ProjectSecurityOverview, String> {
        let repo = SnapshotRepository::new(self.db.clone());

        // Get all snapshots for this project
        let filter = SnapshotFilter {
            project_path: Some(project_path.to_string()),
            ..Default::default()
        };
        let snapshots = repo.list_snapshots(&filter)?;

        if snapshots.is_empty() {
            return Ok(ProjectSecurityOverview {
                project_path: project_path.to_string(),
                risk_score: 0,
                risk_level: OverallRiskLevel::Low,
                total_snapshots: 0,
                latest_snapshot_id: None,
                latest_snapshot_date: None,
                insight_summary: InsightSummary::default(),
                typosquatting_alerts: Vec::new(),
                frequent_updaters: Vec::new(),
                dependency_health: Vec::new(),
            });
        }

        // Get latest snapshot
        let latest = snapshots.first().unwrap();
        let latest_insight_summary = repo.get_insight_summary(&latest.id)?;

        // Collect all insights from recent snapshots (last 10)
        let recent_snapshots = snapshots.iter().take(10).collect::<Vec<_>>();
        let mut all_insights = Vec::new();
        for snapshot in &recent_snapshots {
            if let Ok(insights) = repo.list_insights(&snapshot.id) {
                all_insights.extend(insights);
            }
        }

        // Aggregate typosquatting alerts
        let typosquatting_alerts = self.aggregate_typosquatting_alerts(&all_insights);

        // Detect frequent updaters from snapshot history
        let frequent_updaters = self.detect_frequent_updaters(&repo, project_path)?;

        // Calculate dependency health for latest snapshot
        let dependency_health = if let Some(snapshot_with_deps) =
            repo.get_snapshot_with_dependencies(&latest.id)?
        {
            self.calculate_dependency_health(&snapshot_with_deps.dependencies, &all_insights)
        } else {
            Vec::new()
        };

        // Calculate overall risk score
        let risk_score = self.calculate_risk_score(&latest_insight_summary, &typosquatting_alerts, &frequent_updaters);
        let risk_level = OverallRiskLevel::from(risk_score);

        Ok(ProjectSecurityOverview {
            project_path: project_path.to_string(),
            risk_score,
            risk_level,
            total_snapshots: snapshots.len(),
            latest_snapshot_id: Some(latest.id.clone()),
            latest_snapshot_date: Some(latest.created_at.clone()),
            insight_summary: latest_insight_summary,
            typosquatting_alerts,
            frequent_updaters,
            dependency_health,
        })
    }

    /// Aggregate typosquatting alerts from insights
    fn aggregate_typosquatting_alerts(&self, insights: &[SecurityInsight]) -> Vec<TyposquattingAlertInfo> {
        let mut alerts = HashMap::new();

        for insight in insights {
            if insight.insight_type == InsightType::TyposquattingSuspect {
                if let Some(package_name) = &insight.package_name {
                    // Only keep first occurrence
                    alerts.entry(package_name.clone()).or_insert_with(|| {
                        let similar_to = insight.metadata
                            .as_ref()
                            .and_then(|m| m.get("similar_to"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        TyposquattingAlertInfo {
                            package_name: package_name.clone(),
                            similar_to,
                            first_seen: insight.created_at.clone(),
                            snapshot_id: insight.snapshot_id.clone(),
                        }
                    });
                }
            }
        }

        let mut result: Vec<_> = alerts.into_values().collect();
        result.sort_by(|a, b| a.package_name.cmp(&b.package_name));
        result
    }

    /// Detect packages that update frequently
    fn detect_frequent_updaters(
        &self,
        repo: &SnapshotRepository,
        project_path: &str,
    ) -> Result<Vec<FrequentUpdater>, String> {
        // Get snapshots from last 30 days
        let filter = SnapshotFilter {
            project_path: Some(project_path.to_string()),
            limit: Some(50),
            ..Default::default()
        };
        let snapshots = repo.list_snapshots(&filter)?;

        if snapshots.len() < 2 {
            return Ok(Vec::new());
        }

        // Track version history per package
        let mut package_versions: HashMap<String, Vec<(String, String)>> = HashMap::new();

        for snapshot in &snapshots {
            if let Some(snapshot_with_deps) = repo.get_snapshot_with_dependencies(&snapshot.id)? {
                for dep in &snapshot_with_deps.dependencies {
                    package_versions
                        .entry(dep.name.clone())
                        .or_default()
                        .push((dep.version.clone(), snapshot.created_at.clone()));
                }
            }
        }

        // Find packages with multiple different versions
        let mut frequent_updaters = Vec::new();
        for (package_name, versions) in package_versions {
            let unique_versions: Vec<_> = versions
                .iter()
                .map(|(v, _)| v.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            // If package has 3+ different versions across snapshots, it's a frequent updater
            if unique_versions.len() >= 3 {
                frequent_updaters.push(FrequentUpdater {
                    package_name,
                    update_count: unique_versions.len() as i32,
                    time_span_days: 30, // Approximate
                    versions: unique_versions,
                });
            }
        }

        // Sort by update count descending
        frequent_updaters.sort_by(|a, b| b.update_count.cmp(&a.update_count));

        Ok(frequent_updaters.into_iter().take(10).collect())
    }

    /// Calculate dependency health scores
    fn calculate_dependency_health(
        &self,
        dependencies: &[crate::models::snapshot::SnapshotDependency],
        insights: &[SecurityInsight],
    ) -> Vec<DependencyHealth> {
        let mut health_list = Vec::new();

        // Group insights by package
        let mut package_insights: HashMap<&str, Vec<&SecurityInsight>> = HashMap::new();
        for insight in insights {
            if let Some(pkg) = &insight.package_name {
                package_insights.entry(pkg.as_str()).or_default().push(insight);
            }
        }

        // Calculate health for each dependency
        for dep in dependencies {
            let insights = package_insights.get(dep.name.as_str());
            let mut factors = Vec::new();
            let mut total_score = 0;
            let mut max_score = 0;

            // Factor 1: No security issues (30 points)
            max_score += 30;
            let security_issues = insights
                .map(|i| i.iter().filter(|x| x.severity >= InsightSeverity::Medium).count())
                .unwrap_or(0);
            let security_score = if security_issues == 0 {
                30
            } else if security_issues == 1 {
                15
            } else {
                0
            };
            total_score += security_score;
            factors.push(HealthFactor {
                name: "Security".to_string(),
                score: security_score,
                max_score: 30,
                description: if security_issues == 0 {
                    "No security issues detected".to_string()
                } else {
                    format!("{} security issue(s) detected", security_issues)
                },
            });

            // Factor 2: No postinstall script (25 points)
            max_score += 25;
            let postinstall_score = if dep.has_postinstall { 0 } else { 25 };
            total_score += postinstall_score;
            factors.push(HealthFactor {
                name: "Postinstall".to_string(),
                score: postinstall_score,
                max_score: 25,
                description: if dep.has_postinstall {
                    "Has postinstall script".to_string()
                } else {
                    "No postinstall script".to_string()
                },
            });

            // Factor 3: Direct dependency (20 points)
            max_score += 20;
            let direct_score = if dep.is_direct { 20 } else { 15 };
            total_score += direct_score;
            factors.push(HealthFactor {
                name: "Dependency Type".to_string(),
                score: direct_score,
                max_score: 20,
                description: if dep.is_direct {
                    "Direct dependency".to_string()
                } else {
                    "Transitive dependency".to_string()
                },
            });

            // Factor 4: No typosquatting suspicion (25 points)
            max_score += 25;
            let typo_issues = insights
                .map(|i| {
                    i.iter()
                        .filter(|x| x.insight_type == InsightType::TyposquattingSuspect)
                        .count()
                })
                .unwrap_or(0);
            let typo_score = if typo_issues == 0 { 25 } else { 0 };
            total_score += typo_score;
            factors.push(HealthFactor {
                name: "Typosquatting".to_string(),
                score: typo_score,
                max_score: 25,
                description: if typo_issues == 0 {
                    "No typosquatting concerns".to_string()
                } else {
                    "Potential typosquatting detected".to_string()
                },
            });

            // Calculate percentage
            let health_score = if max_score > 0 {
                (total_score * 100) / max_score
            } else {
                100
            };

            health_list.push(DependencyHealth {
                package_name: dep.name.clone(),
                version: dep.version.clone(),
                health_score,
                factors,
            });
        }

        // Sort by health score ascending (worst first)
        health_list.sort_by(|a, b| a.health_score.cmp(&b.health_score));

        health_list
    }

    /// Calculate overall project risk score (0-100)
    fn calculate_risk_score(
        &self,
        insight_summary: &InsightSummary,
        typosquatting_alerts: &[TyposquattingAlertInfo],
        frequent_updaters: &[FrequentUpdater],
    ) -> i32 {
        let mut score = 0;

        // Critical insights: +40 points each (max 80)
        score += std::cmp::min(insight_summary.critical as i32 * 40, 80);

        // High insights: +15 points each (max 45)
        score += std::cmp::min(insight_summary.high as i32 * 15, 45);

        // Medium insights: +5 points each (max 15)
        score += std::cmp::min(insight_summary.medium as i32 * 5, 15);

        // Typosquatting alerts: +25 points each (max 50)
        score += std::cmp::min(typosquatting_alerts.len() as i32 * 25, 50);

        // Frequent updaters: +3 points each (max 15)
        score += std::cmp::min(frequent_updaters.len() as i32 * 3, 15);

        // Cap at 100
        std::cmp::min(score, 100)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_from_score() {
        assert_eq!(OverallRiskLevel::from(0), OverallRiskLevel::Low);
        assert_eq!(OverallRiskLevel::from(25), OverallRiskLevel::Low);
        assert_eq!(OverallRiskLevel::from(26), OverallRiskLevel::Medium);
        assert_eq!(OverallRiskLevel::from(50), OverallRiskLevel::Medium);
        assert_eq!(OverallRiskLevel::from(51), OverallRiskLevel::High);
        assert_eq!(OverallRiskLevel::from(75), OverallRiskLevel::High);
        assert_eq!(OverallRiskLevel::from(76), OverallRiskLevel::Critical);
        assert_eq!(OverallRiskLevel::from(100), OverallRiskLevel::Critical);
    }
}
