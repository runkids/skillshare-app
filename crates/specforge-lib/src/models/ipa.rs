// IPA metadata models
// Represents IPA file metadata (read-only)

use serde::{Deserialize, Serialize};

/// Represents IPA file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpaMetadata {
    pub file_name: String,
    pub file_path: String,
    pub bundle_id: String,
    pub version: String,
    pub build: String,
    pub display_name: String,
    pub device_capabilities: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_plist: Option<serde_json::Value>,
    pub created_at: String,
}

impl IpaMetadata {
    pub fn new(file_name: String, file_path: String) -> Self {
        Self {
            file_name,
            file_path,
            bundle_id: String::new(),
            version: String::new(),
            build: String::new(),
            display_name: String::new(),
            device_capabilities: String::new(),
            error: None,
            full_plist: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_error(file_name: String, file_path: String, error: String) -> Self {
        Self {
            file_name,
            file_path,
            bundle_id: String::new(),
            version: String::new(),
            build: String::new(),
            display_name: String::new(),
            device_capabilities: String::new(),
            error: Some(error),
            full_plist: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
