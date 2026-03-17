// Schema data model
// A Schema defines the structure of a Spec: its fields and required sections.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinition {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub fields: HashMap<String, FieldType>,
    #[serde(default)]
    pub sections: Vec<SectionDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldType {
    /// Simple type: "string", "number", "date", "list"
    Simple(String),
    /// Enum type: list of allowed string values, e.g. ["high", "medium", "low"]
    Enum(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionDef {
    pub name: String,
    #[serde(default)]
    pub required: bool,
}

impl SchemaDefinition {
    /// Parse a SchemaDefinition from YAML content.
    pub fn from_yaml(content: &str) -> Result<Self, String> {
        serde_yaml::from_str(content).map_err(|e| format!("Failed to parse schema YAML: {e}"))
    }

    /// Validate fields against this schema definition.
    ///
    /// Returns a list of validation error messages. An empty list means all valid.
    pub fn validate_fields(
        &self,
        fields: &HashMap<String, serde_yaml::Value>,
    ) -> Vec<String> {
        let mut errors = Vec::new();

        for (field_name, field_value) in fields {
            if let Some(field_type) = self.fields.get(field_name) {
                match field_type {
                    FieldType::Simple(type_name) => match type_name.as_str() {
                        "string" => {
                            if !field_value.is_string() {
                                errors.push(format!(
                                    "Field '{field_name}' must be a string"
                                ));
                            }
                        }
                        "number" => {
                            if !field_value.is_number() {
                                errors.push(format!(
                                    "Field '{field_name}' must be a number"
                                ));
                            }
                        }
                        "date" => {
                            if let Some(s) = field_value.as_str() {
                                if !is_valid_date(s) {
                                    errors.push(format!(
                                        "Field '{field_name}' must be a valid date (YYYY-MM-DD)"
                                    ));
                                }
                            } else {
                                errors.push(format!(
                                    "Field '{field_name}' must be a string in YYYY-MM-DD format"
                                ));
                            }
                        }
                        "list" => {
                            if !field_value.is_sequence() {
                                errors.push(format!(
                                    "Field '{field_name}' must be a list"
                                ));
                            }
                        }
                        _ => {
                            // Unknown simple type, skip validation
                        }
                    },
                    FieldType::Enum(allowed) => {
                        if let Some(s) = field_value.as_str() {
                            if !allowed.contains(&s.to_string()) {
                                errors.push(format!(
                                    "Field '{field_name}' must be one of: {}",
                                    allowed.join(", ")
                                ));
                            }
                        } else {
                            errors.push(format!(
                                "Field '{field_name}' must be a string (one of: {})",
                                allowed.join(", ")
                            ));
                        }
                    }
                }
            }
            // Fields not in schema are silently allowed (extensible)
        }

        errors
    }
}

/// Check if a string matches YYYY-MM-DD format (ISO 8601 date).
fn is_valid_date(s: &str) -> bool {
    if s.len() != 10 {
        return false;
    }
    // Validate format with chrono
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA_YAML: &str = r#"
name: change-request
display_name: "Change Request"
description: "A request to change existing functionality"
fields:
  title: string
  priority: [high, medium, low]
  assignee: string
  tags: list
  deadline: date
  story_points: number
sections:
  - name: summary
    required: true
  - name: acceptance-criteria
    required: true
  - name: technical-notes
    required: false
"#;

    #[test]
    fn test_parse_schema_yaml() {
        let schema = SchemaDefinition::from_yaml(SCHEMA_YAML).expect("should parse");
        assert_eq!(schema.name, "change-request");
        assert_eq!(
            schema.display_name,
            Some("Change Request".to_string())
        );
        assert_eq!(
            schema.description,
            Some("A request to change existing functionality".to_string())
        );

        // Check field types
        assert!(matches!(
            schema.fields.get("title"),
            Some(FieldType::Simple(s)) if s == "string"
        ));
        assert!(matches!(
            schema.fields.get("priority"),
            Some(FieldType::Enum(v)) if v == &vec!["high".to_string(), "medium".to_string(), "low".to_string()]
        ));
        assert!(matches!(
            schema.fields.get("tags"),
            Some(FieldType::Simple(s)) if s == "list"
        ));
        assert!(matches!(
            schema.fields.get("deadline"),
            Some(FieldType::Simple(s)) if s == "date"
        ));
        assert!(matches!(
            schema.fields.get("story_points"),
            Some(FieldType::Simple(s)) if s == "number"
        ));

        // Check sections
        assert_eq!(schema.sections.len(), 3);
        assert_eq!(schema.sections[0].name, "summary");
        assert!(schema.sections[0].required);
        assert_eq!(schema.sections[1].name, "acceptance-criteria");
        assert!(schema.sections[1].required);
        assert_eq!(schema.sections[2].name, "technical-notes");
        assert!(!schema.sections[2].required);
    }

    #[test]
    fn test_validate_fields_valid() {
        let schema = SchemaDefinition::from_yaml(SCHEMA_YAML).expect("should parse");
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            serde_yaml::Value::String("My Title".to_string()),
        );
        fields.insert(
            "priority".to_string(),
            serde_yaml::Value::String("high".to_string()),
        );
        fields.insert(
            "assignee".to_string(),
            serde_yaml::Value::String("willie".to_string()),
        );
        fields.insert(
            "tags".to_string(),
            serde_yaml::Value::Sequence(vec![
                serde_yaml::Value::String("auth".to_string()),
                serde_yaml::Value::String("security".to_string()),
            ]),
        );
        fields.insert(
            "deadline".to_string(),
            serde_yaml::Value::String("2026-03-17".to_string()),
        );
        fields.insert(
            "story_points".to_string(),
            serde_yaml::Value::Number(serde_yaml::Number::from(5)),
        );

        let errors = schema.validate_fields(&fields);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_validate_fields_invalid_enum() {
        let schema = SchemaDefinition::from_yaml(SCHEMA_YAML).expect("should parse");
        let mut fields = HashMap::new();
        fields.insert(
            "priority".to_string(),
            serde_yaml::Value::String("urgent".to_string()),
        );

        let errors = schema.validate_fields(&fields);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("must be one of"));
        assert!(errors[0].contains("high"));
        assert!(errors[0].contains("medium"));
        assert!(errors[0].contains("low"));
    }

    #[test]
    fn test_validate_fields_invalid_type() {
        let schema = SchemaDefinition::from_yaml(SCHEMA_YAML).expect("should parse");
        let mut fields = HashMap::new();
        // title should be string, not a number
        fields.insert(
            "title".to_string(),
            serde_yaml::Value::Number(serde_yaml::Number::from(42)),
        );
        // tags should be a list, not a string
        fields.insert(
            "tags".to_string(),
            serde_yaml::Value::String("auth".to_string()),
        );
        // story_points should be number, not a string
        fields.insert(
            "story_points".to_string(),
            serde_yaml::Value::String("five".to_string()),
        );

        let errors = schema.validate_fields(&fields);
        assert_eq!(errors.len(), 3);

        let errors_joined = errors.join("; ");
        assert!(errors_joined.contains("title"));
        assert!(errors_joined.contains("tags"));
        assert!(errors_joined.contains("story_points"));
    }

    #[test]
    fn test_validate_date_format() {
        let schema = SchemaDefinition::from_yaml(SCHEMA_YAML).expect("should parse");

        // Valid date
        let mut fields = HashMap::new();
        fields.insert(
            "deadline".to_string(),
            serde_yaml::Value::String("2026-03-17".to_string()),
        );
        let errors = schema.validate_fields(&fields);
        assert!(errors.is_empty());

        // Invalid date format
        let mut fields = HashMap::new();
        fields.insert(
            "deadline".to_string(),
            serde_yaml::Value::String("03/17/2026".to_string()),
        );
        let errors = schema.validate_fields(&fields);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("YYYY-MM-DD"));

        // Invalid date (not a string)
        let mut fields = HashMap::new();
        fields.insert(
            "deadline".to_string(),
            serde_yaml::Value::Number(serde_yaml::Number::from(20260317)),
        );
        let errors = schema.validate_fields(&fields);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("YYYY-MM-DD"));
    }
}
