// Step Template models for workflow step templates
// Supports custom templates saved by users

use serde::{Deserialize, Serialize};

/// Template category identifiers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateCategory {
    PackageManager,
    Git,
    Docker,
    Shell,
    Testing,
    CodeQuality,
    Kubernetes,
    Database,
    Cloud,
    Ai,
    Security,
    Nodejs,
    Custom,
}

/// Custom template saved by user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomStepTemplate {
    pub id: String,
    pub name: String,
    pub command: String,
    pub category: TemplateCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_custom: bool,
    pub created_at: String,
}

/// Response for list custom templates
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCustomTemplatesResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates: Option<Vec<CustomStepTemplate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for save/delete operations
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomTemplateResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<CustomStepTemplate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
