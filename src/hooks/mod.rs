//! Hook type system for Claude Code and OpenCode hooks.
//!
//! Each hook type (Stop, Notification, PreToolUse, etc.) has specific response requirements.
//! This module provides a trait-based abstraction for handling different hook types.
//!
//! OpenCode events are automatically detected and mapped to internal hook types.
//! See the [`opencode`] module for the mapping table.

pub mod compact;
pub mod notification;
pub mod opencode;
pub mod permission;
pub mod prompt;
pub mod session;
pub mod stop;
pub mod tool_use;

use crate::event::Event;
use anyhow::{bail, Result};
use serde_json::Value;

/// Outcome from executing a notification handler
#[derive(Debug, Clone)]
pub enum HandlerOutcome {
    /// Handler succeeded
    Success,
    /// Handler failed with an error
    Error(String),
    /// Handler requires user interaction (future: for PreToolUse)
    #[allow(dead_code)]
    Interactive(InteractiveResponse),
}

/// Interactive response from a handler (for PreToolUse hooks)
#[derive(Debug, Clone)]
pub struct InteractiveResponse {
    pub decision: PermissionDecision,
    pub reason: Option<String>,
}

/// Permission decision for tool use
#[derive(Debug, Clone)]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

/// Trait for hook types (Claude Code and OpenCode).
///
/// Each hook type knows how to generate its own JSON response format.
pub trait Hook: Send + Sync {
    /// Get the hook type name (e.g., "Stop", "PreToolUse")
    fn hook_type(&self) -> &str;

    /// Generate the appropriate JSON response based on handler outcomes
    fn generate_response(&self, outcomes: &[HandlerOutcome]) -> Value;
}

/// Create a Hook instance from an event.
///
/// Supports both Claude Code events (via `hook_event_name` field) and OpenCode events
/// (via `type`, `event`, or `hook` fields with dotted notation like `tool.execute.before`).
///
/// OpenCode events are normalized: a `hook_event_name` field is injected into the event
/// data so that existing matchers work without modification.
pub fn hook_from_event(event: &Event) -> Result<Box<dyn Hook>> {
    // Try Claude Code format first
    let hook_event_name = if let Some(name) = event.get_str("hook_event_name") {
        name.to_string()
    } else if let Some(oc_event) = opencode::detect_opencode_event_type(&event.data) {
        // Map OpenCode event to internal hook name
        match opencode::map_opencode_event(&oc_event) {
            Some(mapped) => mapped.to_string(),
            None => bail!("Unrecognized OpenCode event: {}", oc_event),
        }
    } else {
        bail!("No hook_event_name or recognized OpenCode event type found")
    };

    hook_from_name(&hook_event_name, event)
}

/// Create a Hook from a resolved hook name and event data.
fn hook_from_name(hook_event_name: &str, event: &Event) -> Result<Box<dyn Hook>> {
    match hook_event_name {
        "Stop" | "SubagentStop" => Ok(Box::new(stop::StopHook::new(hook_event_name))),
        "Notification" => Ok(Box::new(notification::NotificationHook)),
        "PreToolUse" => Ok(Box::new(tool_use::PreToolUseHook::from_event(event)?)),
        "PostToolUse" => Ok(Box::new(tool_use::PostToolUseHook)),
        "PermissionRequest" => Ok(Box::new(permission::PermissionRequestHook)),
        "UserPromptSubmit" => Ok(Box::new(prompt::UserPromptSubmitHook)),
        "SessionStart" => Ok(Box::new(session::SessionStartHook)),
        "SessionEnd" => Ok(Box::new(session::SessionEndHook)),
        "PreCompact" => Ok(Box::new(compact::PreCompactHook)),
        "FileEdited" => Ok(Box::new(opencode::FileEditedHook)),
        "SessionError" => Ok(Box::new(opencode::SessionErrorHook)),
        _ => bail!("Unknown hook type: {}", hook_event_name),
    }
}
