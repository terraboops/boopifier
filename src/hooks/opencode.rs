//! OpenCode event normalization and hook support.
//!
//! OpenCode uses a different event naming convention than Claude Code.
//! This module maps OpenCode event names to internal hook types so that
//! existing matchers and handlers work unchanged.
//!
//! ## Event Name Mapping
//!
//! | OpenCode Event          | Internal Hook Type |
//! |-------------------------|--------------------|
//! | `tool.execute.before`   | `PreToolUse`       |
//! | `tool.execute.after`    | `PostToolUse`      |
//! | `session.idle`          | `Stop`             |
//! | `session.created`       | `SessionStart`     |
//! | `session.deleted`       | `SessionEnd`       |
//! | `session.completed`     | `Stop`             |
//! | `session.compacted`     | `PreCompact`       |
//! | `session.compacting`    | `PreCompact`       |
//! | `file.edited`           | `FileEdited`       |
//! | `session.error`         | `SessionError`     |

use super::{HandlerOutcome, Hook};
use serde_json::{json, Value};

/// Maps an OpenCode event type string to the equivalent internal hook name.
///
/// Returns `None` if the event type is not a recognized OpenCode event.
pub fn map_opencode_event(event_type: &str) -> Option<&'static str> {
    match event_type {
        "tool.execute.before" => Some("PreToolUse"),
        "tool.execute.after" => Some("PostToolUse"),
        "session.idle" => Some("Stop"),
        "session.created" => Some("SessionStart"),
        "session.deleted" => Some("SessionEnd"),
        "session.completed" => Some("Stop"),
        "session.compacted" | "session.compacting" => Some("PreCompact"),
        "file.edited" => Some("FileEdited"),
        "session.error" => Some("SessionError"),
        _ => None,
    }
}

/// Detects whether a JSON event is from OpenCode by checking for known fields.
///
/// OpenCode events typically have a `type` or `event` field with dotted notation
/// (e.g., `tool.execute.before`), whereas Claude Code events use `hook_event_name`.
pub fn detect_opencode_event_type(
    data: &std::collections::HashMap<String, Value>,
) -> Option<String> {
    // Check common OpenCode event type fields
    for field in &["type", "event", "hook"] {
        if let Some(Value::String(s)) = data.get(*field) {
            if s.contains('.') && map_opencode_event(s).is_some() {
                return Some(s.clone());
            }
        }
    }
    None
}

/// Handler for FileEdited hooks (OpenCode-only).
///
/// Fires when OpenCode detects a file has been edited.
/// Returns an empty object for passive observation.
pub struct FileEditedHook;

impl Hook for FileEditedHook {
    fn hook_type(&self) -> &str {
        "FileEdited"
    }

    fn generate_response(&self, _outcomes: &[HandlerOutcome]) -> Value {
        json!({})
    }
}

/// Handler for SessionError hooks (OpenCode-only).
///
/// Fires when an OpenCode session encounters an error.
/// Returns an empty object for passive observation.
pub struct SessionErrorHook;

impl Hook for SessionErrorHook {
    fn hook_type(&self) -> &str {
        "SessionError"
    }

    fn generate_response(&self, _outcomes: &[HandlerOutcome]) -> Value {
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_known_opencode_events() {
        assert_eq!(
            map_opencode_event("tool.execute.before"),
            Some("PreToolUse")
        );
        assert_eq!(
            map_opencode_event("tool.execute.after"),
            Some("PostToolUse")
        );
        assert_eq!(map_opencode_event("session.idle"), Some("Stop"));
        assert_eq!(map_opencode_event("session.created"), Some("SessionStart"));
        assert_eq!(map_opencode_event("session.deleted"), Some("SessionEnd"));
        assert_eq!(map_opencode_event("session.completed"), Some("Stop"));
        assert_eq!(map_opencode_event("session.compacted"), Some("PreCompact"));
        assert_eq!(map_opencode_event("session.compacting"), Some("PreCompact"));
        assert_eq!(map_opencode_event("file.edited"), Some("FileEdited"));
        assert_eq!(map_opencode_event("session.error"), Some("SessionError"));
    }

    #[test]
    fn test_map_unknown_event() {
        assert_eq!(map_opencode_event("unknown.event"), None);
        assert_eq!(map_opencode_event("Stop"), None);
    }

    #[test]
    fn test_detect_opencode_event_from_type_field() {
        let mut data = std::collections::HashMap::new();
        data.insert("type".to_string(), json!("tool.execute.before"));
        assert_eq!(
            detect_opencode_event_type(&data),
            Some("tool.execute.before".to_string())
        );
    }

    #[test]
    fn test_detect_opencode_event_from_event_field() {
        let mut data = std::collections::HashMap::new();
        data.insert("event".to_string(), json!("session.idle"));
        assert_eq!(
            detect_opencode_event_type(&data),
            Some("session.idle".to_string())
        );
    }

    #[test]
    fn test_detect_opencode_event_from_hook_field() {
        let mut data = std::collections::HashMap::new();
        data.insert("hook".to_string(), json!("file.edited"));
        assert_eq!(
            detect_opencode_event_type(&data),
            Some("file.edited".to_string())
        );
    }

    #[test]
    fn test_detect_no_opencode_event_for_claude_code() {
        let mut data = std::collections::HashMap::new();
        data.insert("hook_event_name".to_string(), json!("Stop"));
        assert_eq!(detect_opencode_event_type(&data), None);
    }

    #[test]
    fn test_detect_no_opencode_event_for_unknown_dotted() {
        let mut data = std::collections::HashMap::new();
        data.insert("type".to_string(), json!("unknown.thing"));
        assert_eq!(detect_opencode_event_type(&data), None);
    }

    #[test]
    fn test_file_edited_hook_response() {
        let hook = FileEditedHook;
        assert_eq!(hook.hook_type(), "FileEdited");
        assert_eq!(hook.generate_response(&[]), json!({}));
    }

    #[test]
    fn test_session_error_hook_response() {
        let hook = SessionErrorHook;
        assert_eq!(hook.hook_type(), "SessionError");
        assert_eq!(hook.generate_response(&[]), json!({}));
    }
}
