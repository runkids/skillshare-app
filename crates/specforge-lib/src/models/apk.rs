// APK metadata models
// Represents APK file metadata (read-only)

use serde::{Deserialize, Serialize};

/// Represents APK file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApkMetadata {
    pub file_name: String,
    pub file_path: String,
    pub package_name: String,
    pub version_name: String,
    pub version_code: String,
    pub app_name: String,
    pub min_sdk: String,
    pub target_sdk: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: String,
    /// File size in bytes
    pub file_size: u64,
}

impl ApkMetadata {
    pub fn new(file_name: String, file_path: String) -> Self {
        Self {
            file_name,
            file_path,
            package_name: String::new(),
            version_name: String::new(),
            version_code: String::new(),
            app_name: String::new(),
            min_sdk: String::new(),
            target_sdk: String::new(),
            error: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            file_size: 0,
        }
    }

    pub fn with_error(file_name: String, file_path: String, error: String) -> Self {
        Self {
            file_name,
            file_path,
            package_name: String::new(),
            version_name: String::new(),
            version_code: String::new(),
            app_name: String::new(),
            min_sdk: String::new(),
            target_sdk: String::new(),
            error: Some(error),
            created_at: chrono::Utc::now().to_rfc3339(),
            file_size: 0,
        }
    }
}
