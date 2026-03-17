// Spec data model
// A Spec is a markdown file with YAML frontmatter containing structured fields.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: String,
    pub schema: String,
    pub title: String,
    #[serde(default = "default_status")]
    pub status: String,
    pub workflow: Option<String>,
    pub workflow_phase: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub fields: HashMap<String, serde_yaml::Value>,
    #[serde(skip)]
    pub body: String,
    #[serde(skip)]
    pub file_path: Option<String>,
}

fn default_status() -> String {
    "draft".to_string()
}

impl Spec {
    /// Generate a spec ID with format: `spec-{YYYY-MM-DD}-{slug}-{4-hex-random}`
    pub fn generate_id(title: &str) -> String {
        let date = chrono::Utc::now().format("%Y-%m-%d");
        let slug = title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>();
        // Collapse consecutive dashes and trim them
        let slug = slug
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        // Truncate slug to keep IDs reasonable
        let slug: String = slug.chars().take(40).collect();
        let slug = slug.trim_end_matches('-');

        let mut rng = rand::thread_rng();
        let hex: String = format!("{:04x}", rng.gen_range(0..0x10000u32));

        format!("spec-{date}-{slug}-{hex}")
    }

    /// Parse a Spec from markdown content with YAML frontmatter.
    ///
    /// Frontmatter is delimited by `---` at the start and end.
    /// Everything after the closing `---` is the body.
    pub fn from_markdown(content: &str) -> Result<Self, String> {
        let trimmed = content.trim_start();

        if !trimmed.starts_with("---") {
            return Err("Missing frontmatter: content must start with '---'".to_string());
        }

        // Find the end of frontmatter (second ---)
        let after_first = &trimmed[3..];
        let end_idx = after_first
            .find("\n---")
            .ok_or("Missing closing '---' for frontmatter")?;

        let yaml_content = &after_first[..end_idx];
        let body_start = end_idx + 4; // skip "\n---"
        let body = if body_start < after_first.len() {
            after_first[body_start..].trim().to_string()
        } else {
            String::new()
        };

        let mut spec: Spec = serde_yaml::from_str(yaml_content)
            .map_err(|e| format!("Failed to parse frontmatter YAML: {e}"))?;
        spec.body = body;

        Ok(spec)
    }

    /// Serialize the Spec back to markdown with YAML frontmatter.
    pub fn to_markdown(&self) -> Result<String, String> {
        let yaml =
            serde_yaml::to_string(self).map_err(|e| format!("Failed to serialize YAML: {e}"))?;

        let mut result = String::from("---\n");
        result.push_str(&yaml);
        result.push_str("---\n");
        if !self.body.is_empty() {
            result.push('\n');
            result.push_str(&self.body);
            result.push('\n');
        }

        Ok(result)
    }

    /// Parse `## Heading` sections from markdown body.
    ///
    /// Returns a map of lowercase-dashed heading name to section content.
    /// For example, `## Acceptance Criteria` becomes key `"acceptance-criteria"`.
    pub fn extract_sections(body: &str) -> HashMap<String, String> {
        let mut sections = HashMap::new();
        let mut current_heading: Option<String> = None;
        let mut current_content = String::new();

        for line in body.lines() {
            if let Some(heading_text) = line.strip_prefix("## ") {
                // Save previous section
                if let Some(ref heading) = current_heading {
                    sections.insert(heading.clone(), current_content.trim().to_string());
                }
                // Start new section
                let key = heading_text
                    .trim()
                    .to_lowercase()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join("-");
                current_heading = Some(key);
                current_content = String::new();
            } else if current_heading.is_some() {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // Save last section
        if let Some(heading) = current_heading {
            sections.insert(heading, current_content.trim().to_string());
        }

        sections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id = Spec::generate_id("Add OAuth2 Login");
        // Format: spec-YYYY-MM-DD-slug-XXXX
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts[0], "spec");
        // parts[1..4] = YYYY, MM, DD
        assert_eq!(parts[1].len(), 4); // year
        assert_eq!(parts[2].len(), 2); // month
        assert_eq!(parts[3].len(), 2); // day
        // Last part is 4-char hex
        let hex_part = parts.last().expect("should have hex part");
        assert_eq!(hex_part.len(), 4);
        assert!(
            u32::from_str_radix(hex_part, 16).is_ok(),
            "last part should be valid hex"
        );
        // Slug should contain the title words
        assert!(id.contains("add"));
        assert!(id.contains("oauth2"));
        assert!(id.contains("login"));
    }

    #[test]
    fn test_parse_spec_from_markdown() {
        let content = r#"---
id: "spec-2026-03-17-user-auth-a3f1"
schema: "change-request"
title: "Add OAuth2 Login"
status: "draft"
workflow: "default"
workflow_phase: "discuss"
created_at: "2026-03-17T10:00:00Z"
updated_at: "2026-03-17T10:00:00Z"
fields:
  priority: high
  assignee: "willie"
  tags: [auth, security]
---

## Summary

Content here...
"#;
        let spec = Spec::from_markdown(content).expect("should parse");
        assert_eq!(spec.id, "spec-2026-03-17-user-auth-a3f1");
        assert_eq!(spec.schema, "change-request");
        assert_eq!(spec.title, "Add OAuth2 Login");
        assert_eq!(spec.status, "draft");
        assert_eq!(spec.workflow, Some("default".to_string()));
        assert_eq!(spec.workflow_phase, Some("discuss".to_string()));
        assert_eq!(spec.created_at, "2026-03-17T10:00:00Z");
        assert_eq!(spec.updated_at, "2026-03-17T10:00:00Z");

        // Check fields
        assert_eq!(
            spec.fields.get("priority"),
            Some(&serde_yaml::Value::String("high".to_string()))
        );
        assert_eq!(
            spec.fields.get("assignee"),
            Some(&serde_yaml::Value::String("willie".to_string()))
        );
        assert!(spec.fields.get("tags").expect("tags").is_sequence());

        // Check body
        assert!(spec.body.contains("## Summary"));
        assert!(spec.body.contains("Content here..."));
    }

    #[test]
    fn test_parse_spec_missing_frontmatter() {
        let content = "No frontmatter here";
        let result = Spec::from_markdown(content);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Missing frontmatter"));
    }

    #[test]
    fn test_roundtrip_markdown() {
        let content = r#"---
id: "spec-2026-03-17-roundtrip-abcd"
schema: "bug-report"
title: "Fix Login Bug"
status: "in-progress"
created_at: "2026-03-17T10:00:00Z"
updated_at: "2026-03-17T11:00:00Z"
fields:
  severity: critical
---

## Steps to Reproduce

1. Go to login page
2. Click login

## Expected Behavior

Should work
"#;
        let spec1 = Spec::from_markdown(content).expect("parse original");
        let serialized = spec1.to_markdown().expect("serialize");
        let spec2 = Spec::from_markdown(&serialized).expect("parse serialized");

        assert_eq!(spec1.id, spec2.id);
        assert_eq!(spec1.schema, spec2.schema);
        assert_eq!(spec1.title, spec2.title);
        assert_eq!(spec1.status, spec2.status);
        assert_eq!(spec1.created_at, spec2.created_at);
        assert_eq!(spec1.updated_at, spec2.updated_at);
        assert_eq!(spec1.fields, spec2.fields);
        // Body content should be preserved (sections present)
        assert!(spec2.body.contains("Steps to Reproduce"));
        assert!(spec2.body.contains("Expected Behavior"));
    }

    #[test]
    fn test_extract_sections() {
        let body = r#"## Summary

This is the summary.

## Acceptance Criteria

- [ ] Criterion 1
- [ ] Criterion 2

## Technical Notes

Some notes here.
"#;
        let sections = Spec::extract_sections(body);
        assert_eq!(sections.len(), 3);
        assert_eq!(
            sections.get("summary"),
            Some(&"This is the summary.".to_string())
        );
        assert!(sections
            .get("acceptance-criteria")
            .expect("should have acceptance-criteria")
            .contains("Criterion 1"));
        assert!(sections
            .get("technical-notes")
            .expect("should have technical-notes")
            .contains("Some notes here."));
    }

    #[test]
    fn test_spec_with_empty_fields() {
        let content = r#"---
id: "spec-2026-03-17-empty-0000"
schema: "simple"
title: "Empty Fields Spec"
created_at: "2026-03-17T10:00:00Z"
updated_at: "2026-03-17T10:00:00Z"
---

Just a body.
"#;
        let spec = Spec::from_markdown(content).expect("should parse");
        assert!(spec.fields.is_empty());
        assert_eq!(spec.status, "draft"); // default
        assert!(spec.body.contains("Just a body."));
    }
}
