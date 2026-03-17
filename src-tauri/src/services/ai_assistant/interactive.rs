// Interactive Element Parser for AI Assistant
// Feature: Enhanced AI Chat Experience (023-enhanced-ai-chat)
// User Story 3: Interactive Response Elements
//
// Parses special syntax in AI responses:
// - [[navigation:route|label]] - Navigation buttons
// - [[action:prompt|label]] - Action chips that trigger prompts
// - [[entity:type:id|label]] - Entity links (projects, workflows, etc.)

use crate::models::ai_assistant::{InteractiveElement, InteractiveElementType};
use regex::Regex;
use std::sync::LazyLock;
use uuid::Uuid;

// ============================================================================
// Regex Patterns (T063)
// ============================================================================

/// Pattern for navigation elements: [[navigation:route|label]]
static NAVIGATION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\[navigation:([^|]+)\|([^\]]+)\]\]").expect("Invalid navigation regex")
});

/// Pattern for action elements: [[action:prompt|label]]
static ACTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\[action:([^|]+)\|([^\]]+)\]\]").expect("Invalid action regex")
});

/// Pattern for entity elements: [[entity:type:id|label]]
static ENTITY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\[entity:([^:]+):([^|]+)\|([^\]]+)\]\]").expect("Invalid entity regex")
});

/// Combined pattern for all interactive elements
static ALL_ELEMENTS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\[(navigation|action|entity):[^\]]+\]\]").expect("Invalid combined regex")
});

// ============================================================================
// Parsing Functions (T063)
// ============================================================================

/// Parse all interactive elements from content
///
/// # Arguments
/// * `content` - The AI response content to parse
///
/// # Returns
/// A vector of parsed InteractiveElement structs with positions
pub fn parse_interactive_elements(content: &str) -> Vec<InteractiveElement> {
    let mut elements = Vec::new();

    // Parse navigation elements
    for caps in NAVIGATION_PATTERN.captures_iter(content) {
        if let (Some(full_match), Some(route), Some(label)) =
            (caps.get(0), caps.get(1), caps.get(2))
        {
            elements.push(InteractiveElement {
                id: Uuid::new_v4().to_string(),
                element_type: InteractiveElementType::Navigation,
                label: label.as_str().to_string(),
                payload: route.as_str().to_string(),
                requires_confirm: false,
                start_index: full_match.start(),
                end_index: full_match.end(),
            });
        }
    }

    // Parse action elements
    for caps in ACTION_PATTERN.captures_iter(content) {
        if let (Some(full_match), Some(prompt), Some(label)) =
            (caps.get(0), caps.get(1), caps.get(2))
        {
            elements.push(InteractiveElement {
                id: Uuid::new_v4().to_string(),
                element_type: InteractiveElementType::Action,
                label: label.as_str().to_string(),
                payload: prompt.as_str().to_string(),
                requires_confirm: false,
                start_index: full_match.start(),
                end_index: full_match.end(),
            });
        }
    }

    // Parse entity elements: [[entity:type:id|label]] -> payload = "type:id"
    for caps in ENTITY_PATTERN.captures_iter(content) {
        if let (Some(full_match), Some(entity_type), Some(entity_id), Some(label)) =
            (caps.get(0), caps.get(1), caps.get(2), caps.get(3))
        {
            elements.push(InteractiveElement {
                id: Uuid::new_v4().to_string(),
                element_type: InteractiveElementType::Entity,
                label: label.as_str().to_string(),
                payload: format!("{}:{}", entity_type.as_str(), entity_id.as_str()),
                requires_confirm: false,
                start_index: full_match.start(),
                end_index: full_match.end(),
            });
        }
    }

    // Sort by position
    elements.sort_by_key(|e| e.start_index);

    elements
}

/// Get clean content with interactive element markers stripped (T064)
///
/// # Arguments
/// * `content` - The AI response content with markers
///
/// # Returns
/// Content with markers replaced by just the labels
pub fn get_clean_content(content: &str) -> String {
    let mut result = content.to_string();

    // Replace navigation elements with just labels
    result = NAVIGATION_PATTERN
        .replace_all(&result, "$2")
        .to_string();

    // Replace action elements with just labels
    result = ACTION_PATTERN
        .replace_all(&result, "$2")
        .to_string();

    // Replace entity elements with just labels
    result = ENTITY_PATTERN
        .replace_all(&result, "$3")
        .to_string();

    result
}

/// Check if content contains any interactive elements
pub fn has_interactive_elements(content: &str) -> bool {
    ALL_ELEMENTS_PATTERN.is_match(content)
}

/// Get positions of interactive elements in content
/// Returns tuples of (start, end, element)
pub fn get_element_positions(content: &str) -> Vec<(usize, usize, InteractiveElement)> {
    parse_interactive_elements(content)
        .into_iter()
        .map(|e| (e.start_index, e.end_index, e))
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_navigation_element() {
        let content = "Click [[navigation:/projects|Projects]] to view all projects.";
        let elements = parse_interactive_elements(content);

        assert_eq!(elements.len(), 1);
        assert!(matches!(elements[0].element_type, InteractiveElementType::Navigation));
        assert_eq!(elements[0].label, "Projects");
        assert_eq!(elements[0].payload, "/projects");
    }

    #[test]
    fn test_parse_action_element() {
        let content = "Try [[action:run tests|Run Tests]] to execute the test suite.";
        let elements = parse_interactive_elements(content);

        assert_eq!(elements.len(), 1);
        assert!(matches!(elements[0].element_type, InteractiveElementType::Action));
        assert_eq!(elements[0].label, "Run Tests");
        assert_eq!(elements[0].payload, "run tests");
    }

    #[test]
    fn test_parse_entity_element() {
        let content = "Check out [[entity:project:abc123|SpecForge]] for details.";
        let elements = parse_interactive_elements(content);

        assert_eq!(elements.len(), 1);
        assert!(matches!(elements[0].element_type, InteractiveElementType::Entity));
        assert_eq!(elements[0].label, "SpecForge");
        assert_eq!(elements[0].payload, "project:abc123");
    }

    #[test]
    fn test_parse_multiple_elements() {
        let content = "Go to [[navigation:/settings|Settings]] or try [[action:help|Get Help]].";
        let elements = parse_interactive_elements(content);

        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn test_get_clean_content() {
        let content = "Click [[navigation:/projects|Projects]] to view all projects.";
        let clean = get_clean_content(content);

        assert_eq!(clean, "Click Projects to view all projects.");
    }

    #[test]
    fn test_get_clean_content_multiple() {
        let content =
            "Go to [[navigation:/settings|Settings]] or try [[action:help|Get Help]] now.";
        let clean = get_clean_content(content);

        assert_eq!(clean, "Go to Settings or try Get Help now.");
    }

    #[test]
    fn test_has_interactive_elements() {
        assert!(has_interactive_elements(
            "Click [[navigation:/home|Home]]"
        ));
        assert!(has_interactive_elements("Try [[action:test|Test]]"));
        assert!(has_interactive_elements(
            "See [[entity:project:123|Project]]"
        ));
        assert!(!has_interactive_elements("No elements here"));
    }

    #[test]
    fn test_get_element_positions() {
        let content = "Start [[navigation:/home|Home]] middle [[action:test|Test]] end";
        let positions = get_element_positions(content);

        assert_eq!(positions.len(), 2);
        assert!(positions[0].0 < positions[1].0);
    }

    #[test]
    fn test_no_elements() {
        let content = "This is plain text without any interactive elements.";
        let elements = parse_interactive_elements(content);

        assert!(elements.is_empty());
    }

    #[test]
    fn test_element_positions() {
        let content = "Click [[navigation:/home|Home]] here.";
        let elements = parse_interactive_elements(content);

        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].start_index, 6);
        // [[navigation:/home|Home]] = 25 characters, end = 6 + 25 = 31
        assert_eq!(elements[0].end_index, 31);
    }
}
