//! Event types for Claude Code and Cursor hooks.
//!
//! This module defines the event structure received from hooks via stdin.
//! Cursor events are automatically normalized with mapped `hook_event_name`
//! and synthesized fields so that existing matchers work transparently.

use crate::hooks::cursor;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A hook event received from stdin.
///
/// Events are flexible JSON objects that can contain any fields.
/// The event type and other metadata are extracted from the JSON.
///
/// Both Claude Code and Cursor events are supported. Cursor events are
/// automatically normalized: `hook_event_name` is remapped from camelCase
/// to PascalCase, and fields like `tool_name` and `session_id` are
/// synthesized from Cursor-specific equivalents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// The raw JSON value for flexible matching
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

impl Event {
    /// Creates a new event from a JSON string.
    ///
    /// If the event is from Cursor (detected by camelCase hook_event_name),
    /// the event name is remapped to PascalCase and Cursor-specific fields
    /// are normalized.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is invalid.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let mut event: Event =
            serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_str(json))
                .map_err(|e| anyhow::anyhow!("Failed to parse event JSON: {}", e))?;

        // Normalize Cursor events: remap hook_event_name and synthesize fields
        if let Some(cursor_event) = cursor::detect_cursor_event(&event.data) {
            if let Some(mapped) = cursor::map_cursor_event(&cursor_event) {
                event.data.insert(
                    "hook_event_name".to_string(),
                    Value::String(mapped.to_string()),
                );
                cursor::normalize_cursor_fields(&mut event.data, &cursor_event);
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
    fn test_cursor_shell_execution_normalized() {
        let json = r#"{"hook_event_name": "beforeShellExecution", "command": "ls -la", "conversation_id": "conv-abc"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("hook_event_name"), Some("PreToolUse"));
        assert_eq!(event.get_str("tool_name"), Some("Bash"));
        assert_eq!(event.get_str("session_id"), Some("conv-abc"));
        // Original fields preserved
        assert_eq!(event.get_str("command"), Some("ls -la"));
    }

    #[test]
    fn test_cursor_file_edit_normalized() {
        let json = r#"{"hook_event_name": "afterFileEdit", "file_path": "src/main.rs"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("hook_event_name"), Some("PostToolUse"));
        assert_eq!(event.get_str("tool_name"), Some("Edit"));
    }

    #[test]
    fn test_cursor_stop_normalized() {
        let json = r#"{"hook_event_name": "stop", "status": "completed"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("hook_event_name"), Some("Stop"));
    }

    #[test]
    fn test_cursor_pretooluse_preserves_tool_name() {
        let json = r#"{"hook_event_name": "preToolUse", "tool_name": "WebSearch", "conversation_id": "conv-xyz"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("hook_event_name"), Some("PreToolUse"));
        assert_eq!(event.get_str("tool_name"), Some("WebSearch"));
        assert_eq!(event.get_str("session_id"), Some("conv-xyz"));
    }

    #[test]
    fn test_claude_code_event_not_remapped() {
        let json = r#"{"hook_event_name": "Stop", "session_id": "sess-123"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("hook_event_name"), Some("Stop"));
        assert_eq!(event.get_str("session_id"), Some("sess-123"));
    }

    #[test]
    fn test_cursor_session_events_normalized() {
        let start = r#"{"hook_event_name": "sessionStart", "conversation_id": "c1"}"#;
        let event = Event::from_json(start).unwrap();
        assert_eq!(event.get_str("hook_event_name"), Some("SessionStart"));
        assert_eq!(event.get_str("session_id"), Some("c1"));

        let end = r#"{"hook_event_name": "sessionEnd", "conversation_id": "c2"}"#;
        let event = Event::from_json(end).unwrap();
        assert_eq!(event.get_str("hook_event_name"), Some("SessionEnd"));
    }
}
