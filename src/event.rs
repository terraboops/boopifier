//! Event types for Claude Code and OpenCode hooks.
//!
//! This module defines the event structure received from hooks via stdin.
//! OpenCode events are automatically normalized with a `hook_event_name` field
//! so that existing matchers work transparently.

use crate::hooks::opencode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A hook event received from stdin.
///
/// Events are flexible JSON objects that can contain any fields.
/// The event type and other metadata are extracted from the JSON.
///
/// Both Claude Code and OpenCode events are supported. OpenCode events are
/// automatically normalized: a `hook_event_name` field is injected so that
/// existing match rules (e.g., `{"hook_event_name": "Stop"}`) work for both.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// The raw JSON value for flexible matching
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

impl Event {
    /// Creates a new event from a JSON string.
    ///
    /// If the event is from OpenCode (detected by dotted event type fields),
    /// a `hook_event_name` field is injected with the mapped internal name.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is invalid.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let mut event: Event =
            serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_str(json))
                .map_err(|e| anyhow::anyhow!("Failed to parse event JSON: {}", e))?;

        // Normalize OpenCode events: inject hook_event_name if missing
        if !event.data.contains_key("hook_event_name") {
            if let Some(oc_type) = opencode::detect_opencode_event_type(&event.data) {
                if let Some(mapped) = opencode::map_opencode_event(&oc_type) {
                    event.data.insert(
                        "hook_event_name".to_string(),
                        Value::String(mapped.to_string()),
                    );
                }
            }
        }

        Ok(event)
    }

    /// Gets a field value as a string reference.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.data.get(key)?.as_str()
    }

    /// Gets a field value as a string, with nested path support (e.g., "tool.name").
    pub fn get_nested_str(&self, path: &str) -> Option<String> {
        let parts: Vec<&str> = path.split('.').collect();
        let value = Value::Object(
            self.data
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        let mut current = &value;
        for part in parts {
            current = current.get(part)?;
        }

        current.as_str().map(|s| s.to_string())
    }

    /// Gets the entire event data as a reference.
    pub fn as_value(&self) -> Value {
        Value::Object(
            self.data
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event() {
        let json = r#"{"event_type": "task_complete", "status": "success"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("event_type"), Some("task_complete"));
        assert_eq!(event.get_str("status"), Some("success"));
    }

    #[test]
    fn test_nested_access() {
        let json = r#"{"tool": {"name": "bash", "status": "success"}}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_nested_str("tool.name"), Some("bash".to_string()));
    }

    #[test]
    fn test_invalid_json() {
        let json = r#"{"invalid": }"#;
        assert!(Event::from_json(json).is_err());
    }

    #[test]
    fn test_opencode_event_normalization() {
        // OpenCode event with "type" field should get hook_event_name injected
        let json = r#"{"type": "tool.execute.before", "tool": "bash"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("hook_event_name"), Some("PreToolUse"));
        // Original fields are preserved
        assert_eq!(event.get_str("tool"), Some("bash"));
    }

    #[test]
    fn test_opencode_session_idle_normalization() {
        let json = r#"{"event": "session.idle", "sessionID": "abc123"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("hook_event_name"), Some("Stop"));
        assert_eq!(event.get_str("sessionID"), Some("abc123"));
    }

    #[test]
    fn test_opencode_file_edited_normalization() {
        let json = r#"{"hook": "file.edited", "file": "src/main.rs"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("hook_event_name"), Some("FileEdited"));
    }

    #[test]
    fn test_claude_code_event_not_renormalized() {
        // Claude Code events with existing hook_event_name should not be modified
        let json = r#"{"hook_event_name": "Stop", "type": "something"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("hook_event_name"), Some("Stop"));
    }

    #[test]
    fn test_unknown_event_no_normalization() {
        // Unknown events should not get hook_event_name injected
        let json = r#"{"type": "unknown.thing", "data": "test"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("hook_event_name"), None);
    }
}
