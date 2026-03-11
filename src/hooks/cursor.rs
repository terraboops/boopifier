//! Cursor IDE event normalization and hook support.
//!
//! Cursor uses camelCase hook names and some Cursor-specific event names.
//! This module maps them to internal hook types so that existing matchers
//! and handlers work unchanged.
//!
//! ## Event Name Mapping
//!
//! | Cursor Event              | Internal Hook Type |
//! |---------------------------|--------------------|
//! | `preToolUse`              | `PreToolUse`       |
//! | `postToolUse`             | `PostToolUse`      |
//! | `postToolUseFailure`      | `PostToolUse`      |
//! | `beforeShellExecution`    | `PreToolUse`       |
//! | `afterShellExecution`     | `PostToolUse`      |
//! | `beforeMCPExecution`      | `PreToolUse`       |
//! | `afterMCPExecution`       | `PostToolUse`      |
//! | `beforeReadFile`          | `PreToolUse`       |
//! | `afterFileEdit`           | `PostToolUse`      |
//! | `beforeSubmitPrompt`      | `UserPromptSubmit` |
//! | `stop`                    | `Stop`             |
//! | `sessionStart`            | `SessionStart`     |
//! | `sessionEnd`              | `SessionEnd`       |
//! | `subagentStart`           | `SubagentStart`    |
//! | `subagentStop`            | `SubagentStop`     |
//! | `preCompact`              | `PreCompact`       |
//! | `afterAgentResponse`      | `Stop`             |
//!
//! ## Field Normalization
//!
//! Cursor uses different field names for tool and session info.
//! When normalizing, we also inject `tool_name` and `session_id`
//! fields so that webhook templates like `{{tool_name}}` work.

use serde_json::Value;
use std::collections::HashMap;

/// Maps a Cursor hook_event_name to the equivalent internal hook name.
///
/// Returns `None` if the event name is not a recognized Cursor event.
pub fn map_cursor_event(event_name: &str) -> Option<&'static str> {
    match event_name {
        "preToolUse" => Some("PreToolUse"),
        "postToolUse" => Some("PostToolUse"),
        "postToolUseFailure" => Some("PostToolUse"),
        "beforeShellExecution" => Some("PreToolUse"),
        "afterShellExecution" => Some("PostToolUse"),
        "beforeMCPExecution" => Some("PreToolUse"),
        "afterMCPExecution" => Some("PostToolUse"),
        "beforeReadFile" => Some("PreToolUse"),
        "afterFileEdit" => Some("PostToolUse"),
        "beforeSubmitPrompt" => Some("UserPromptSubmit"),
        "stop" => Some("Stop"),
        "sessionStart" => Some("SessionStart"),
        "sessionEnd" => Some("SessionEnd"),
        "subagentStart" => Some("SubagentStart"),
        "subagentStop" => Some("SubagentStop"),
        "preCompact" => Some("PreCompact"),
        "afterAgentResponse" => Some("Stop"),
        "afterAgentThought" => None, // No useful mapping; ignore
        "beforeTabFileRead" | "afterTabFileEdit" => None, // Tab completion hooks; ignore
        _ => None,
    }
}

/// Detects whether a JSON event is from Cursor and returns the event name.
///
/// Cursor events have a `hook_event_name` field with camelCase values
/// (e.g., `beforeShellExecution`), unlike Claude Code's PascalCase.
/// We detect Cursor events by checking if the hook_event_name is camelCase
/// and maps to a known Cursor event.
pub fn detect_cursor_event(data: &HashMap<String, Value>) -> Option<String> {
    if let Some(Value::String(name)) = data.get("hook_event_name") {
        // If it maps to a Cursor event, it's from Cursor
        if map_cursor_event(name).is_some() {
            return Some(name.clone());
        }
    }
    None
}

/// Normalize Cursor-specific fields into the canonical names used by
/// boopifier's webhook templates and matchers.
///
/// Cursor sends tool info in different fields depending on the hook type:
/// - `beforeShellExecution`: command in `command`
/// - `beforeMCPExecution`: tool in `tool_name`
/// - `preToolUse`: tool in `tool_name`
/// - `beforeReadFile`: file in `file_path`
/// - Session info in `conversation_id` (no `session_id`)
pub fn normalize_cursor_fields(data: &mut HashMap<String, Value>, cursor_event: &str) {
    // Map conversation_id -> session_id if session_id is missing
    if !data.contains_key("session_id") {
        if let Some(conv_id) = data.get("conversation_id").cloned() {
            data.insert("session_id".to_string(), conv_id);
        }
    }

    // Synthesize tool_name from Cursor-specific fields
    if !data.contains_key("tool_name") {
        let tool_name = match cursor_event {
            "beforeShellExecution" | "afterShellExecution" => Some("Bash"),
            "beforeReadFile" => Some("Read"),
            "afterFileEdit" => Some("Edit"),
            _ => None,
        };
        if let Some(name) = tool_name {
            data.insert("tool_name".to_string(), Value::String(name.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_map_known_cursor_events() {
        assert_eq!(map_cursor_event("preToolUse"), Some("PreToolUse"));
        assert_eq!(map_cursor_event("postToolUse"), Some("PostToolUse"));
        assert_eq!(
            map_cursor_event("beforeShellExecution"),
            Some("PreToolUse")
        );
        assert_eq!(
            map_cursor_event("afterShellExecution"),
            Some("PostToolUse")
        );
        assert_eq!(
            map_cursor_event("beforeMCPExecution"),
            Some("PreToolUse")
        );
        assert_eq!(map_cursor_event("afterMCPExecution"), Some("PostToolUse"));
        assert_eq!(map_cursor_event("beforeReadFile"), Some("PreToolUse"));
        assert_eq!(map_cursor_event("afterFileEdit"), Some("PostToolUse"));
        assert_eq!(map_cursor_event("stop"), Some("Stop"));
        assert_eq!(map_cursor_event("sessionStart"), Some("SessionStart"));
        assert_eq!(map_cursor_event("sessionEnd"), Some("SessionEnd"));
        assert_eq!(map_cursor_event("subagentStart"), Some("SubagentStart"));
        assert_eq!(map_cursor_event("subagentStop"), Some("SubagentStop"));
        assert_eq!(map_cursor_event("preCompact"), Some("PreCompact"));
        assert_eq!(
            map_cursor_event("beforeSubmitPrompt"),
            Some("UserPromptSubmit")
        );
        assert_eq!(map_cursor_event("afterAgentResponse"), Some("Stop"));
    }

    #[test]
    fn test_map_unknown_event() {
        assert_eq!(map_cursor_event("UnknownEvent"), None);
        assert_eq!(map_cursor_event("Stop"), None); // PascalCase = Claude Code, not Cursor
    }

    #[test]
    fn test_map_ignored_events() {
        assert_eq!(map_cursor_event("afterAgentThought"), None);
        assert_eq!(map_cursor_event("beforeTabFileRead"), None);
        assert_eq!(map_cursor_event("afterTabFileEdit"), None);
    }

    #[test]
    fn test_detect_cursor_event() {
        let mut data = HashMap::new();
        data.insert(
            "hook_event_name".to_string(),
            json!("beforeShellExecution"),
        );
        assert_eq!(
            detect_cursor_event(&data),
            Some("beforeShellExecution".to_string())
        );
    }

    #[test]
    fn test_detect_not_cursor_for_claude_code() {
        let mut data = HashMap::new();
        data.insert("hook_event_name".to_string(), json!("Stop"));
        assert_eq!(detect_cursor_event(&data), None);
    }

    #[test]
    fn test_normalize_shell_execution() {
        let mut data = HashMap::new();
        data.insert(
            "hook_event_name".to_string(),
            json!("beforeShellExecution"),
        );
        data.insert("command".to_string(), json!("ls -la"));
        data.insert("conversation_id".to_string(), json!("conv-123"));

        normalize_cursor_fields(&mut data, "beforeShellExecution");

        assert_eq!(data.get("tool_name").unwrap(), "Bash");
        assert_eq!(data.get("session_id").unwrap(), "conv-123");
    }

    #[test]
    fn test_normalize_read_file() {
        let mut data = HashMap::new();
        data.insert("hook_event_name".to_string(), json!("beforeReadFile"));
        data.insert("file_path".to_string(), json!("src/main.rs"));

        normalize_cursor_fields(&mut data, "beforeReadFile");

        assert_eq!(data.get("tool_name").unwrap(), "Read");
    }

    #[test]
    fn test_normalize_file_edit() {
        let mut data = HashMap::new();
        data.insert("hook_event_name".to_string(), json!("afterFileEdit"));

        normalize_cursor_fields(&mut data, "afterFileEdit");

        assert_eq!(data.get("tool_name").unwrap(), "Edit");
    }

    #[test]
    fn test_normalize_does_not_overwrite_existing() {
        let mut data = HashMap::new();
        data.insert("hook_event_name".to_string(), json!("preToolUse"));
        data.insert("tool_name".to_string(), json!("WebSearch"));
        data.insert("session_id".to_string(), json!("existing-session"));

        normalize_cursor_fields(&mut data, "preToolUse");

        // Should not overwrite existing values
        assert_eq!(data.get("tool_name").unwrap(), "WebSearch");
        assert_eq!(data.get("session_id").unwrap(), "existing-session");
    }
}
