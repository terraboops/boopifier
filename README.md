# Boopifier

A universal notification handler for Claude Code and OpenCode events.

Boopifier reads JSON events from stdin (sent by Claude Code or OpenCode hooks) and dispatches them to various notification handlers. Play sounds when Claude responds, get desktop notifications for important events, send yourself Signal messages, and more. **Crucially, it supports project-specific notification configs in your global config file** - perfect for keeping work notification preferences out of work repos while still getting customized notifications for each project.

## Features

- **Project-Specific Overrides**: Define different notification handlers for different projects (by path pattern) in your global config - keep personal notification preferences out of work repos
- **Cross-Platform Hook Support**: Full implementation of all Claude Code hook types (Stop, Notification, PermissionRequest, SessionStart/End, PreCompact, and more) and OpenCode hook types (tool.execute.before/after, session.idle, file.edited, and more)
- **Multiple Notification Targets**: Desktop, Sound, Signal, Webhook, Email
- **Flexible Event Matching**: Route different hook events to different handlers with regex support
- **Secrets Management**: Environment variables and file-based secrets
- **Async Handler Execution**: Fast, concurrent notification delivery
- **Extensible Plugin System**: Easy to add new notification handlers

## Quick Start

### Installation

```bash
# Via Homebrew (macOS/Linux)
brew tap terraboops/boopifier https://github.com/terraboops/boopifier
brew install boopifier

# Or build from source
make install
```

### Platform Support

| Handler | Linux | macOS | Windows |
|---------|-------|-------|---------|
| `desktop` | ✅ | ⚠️ | ⚠️ |
| `sound` | ✅ | ⚠️ | ⚠️ |
| `webhook` | ✅ | ⚠️ | ⚠️ |
| `email` | ✅ | ⚠️ | ⚠️ |
| `signal` | ✅ (requires signal-cli) | ⚠️ (requires signal-cli) | ❌ |

**Legend:** ✅ Tested | ⚠️ Should work (untested) | ❌ Not supported

### Setup with Claude Code

**Step 1: Configure Claude Code hooks**

See the [Claude Code hooks documentation](https://code.claude.com/docs/en/hooks) for details. Add to your `~/.claude/settings.json`:

```json
{
  "hooks": {
    "Notification": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "boopifier"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "boopifier"
          }
        ]
      }
    ]
  }
}
```

This pipes hook events directly to boopifier.

**Step 2: Configure boopifier handlers**

Create a config file. Boopifier automatically finds it using this priority:
1. `$CLAUDE_PROJECT_DIR/.claude/boopifier.json` (project-specific)
2. `~/.claude/boopifier.json` (global fallback)

You can also specify a custom path with `-c /path/to/config.json`

Example `.claude/boopifier.json`:

```json
{
  "handlers": [
    {
      "name": "play-sound-on-notification",
      "type": "sound",
      "match_rules": {"hook_event_name": "Notification"},
      "config": {
        "file": "/path/to/notification.mp3",
        "volume": 0.8
      }
    },
    {
      "name": "desktop-on-stop",
      "type": "desktop",
      "match_rules": {"hook_event_name": "Stop"},
      "config": {
        "summary": "Claude Code",
        "body": "Agent finished responding"
      }
    }
  ]
}
```

Now boopifier will play a sound on Notification events and show a desktop notification when Claude stops responding!

### Setup with OpenCode

Boopifier natively supports [OpenCode](https://github.com/anomalyco/opencode) hooks. OpenCode events are automatically detected and normalized, so the same match rules and handlers work with both systems.

**Step 1: Configure OpenCode hooks**

Add boopifier as a shell hook in your `opencode.json`:

```json
{
  "hooks": {
    "session_completed": [
      {
        "command": ["boopifier"]
      }
    ],
    "file_edited": [
      {
        "command": ["boopifier"]
      }
    ]
  }
}
```

For plugin-based hooks, you can pipe events to boopifier from an OpenCode plugin using `ctx.$`:

```typescript
export const BoopifierPlugin: Plugin = async ({ $, client }) => ({
  event: async ({ event }) => {
    await $`echo ${JSON.stringify(event)} | boopifier`;
  },
});
```

**Step 2: Configure boopifier handlers**

Create a config file. When running under OpenCode, boopifier looks for config in this order:
1. `$OPENCODE_PROJECT_DIR/.opencode/boopifier.json` (OpenCode project config)
2. `$OPENCODE_PROJECT_DIR/.claude/boopifier.json` (shared project config)
3. `~/.config/opencode/boopifier.json` (OpenCode global config)
4. `~/.claude/boopifier.json` (shared global fallback)

The config format is identical. OpenCode events are normalized with a `hook_event_name` field, so existing match rules work:

```json
{
  "handlers": [
    {
      "name": "sound-on-session-idle",
      "type": "sound",
      "match_rules": {"hook_event_name": "Stop"},
      "config": {
        "file": "/path/to/done.mp3"
      }
    }
  ]
}
```

#### OpenCode Event Mapping

OpenCode events are automatically mapped to internal hook names:

| OpenCode Event            | `hook_event_name` | Description                      |
|---------------------------|-------------------|----------------------------------|
| `tool.execute.before`     | `PreToolUse`      | Before tool execution            |
| `tool.execute.after`      | `PostToolUse`     | After tool execution             |
| `session.idle`            | `Stop`            | Session idle / agent stopped     |
| `session.created`         | `SessionStart`    | New session started              |
| `session.deleted`         | `SessionEnd`      | Session deleted                  |
| `session.completed`       | `Stop`            | Session completed                |
| `session.compacted`       | `PreCompact`      | History was compacted            |
| `session.compacting`      | `PreCompact`      | History is about to be compacted |
| `file.edited`             | `FileEdited`      | A file was edited (OpenCode-only)|
| `session.error`           | `SessionError`    | Session error (OpenCode-only)    |

This means you can write a single boopifier config that works with both Claude Code and OpenCode.

### Project-Specific Overrides

You can define project-specific handler configurations in your **global** config file using path patterns. This is useful for work projects where you don't want to commit personal notification settings to the repo.

Add an `overrides` array to `~/.claude/boopifier.json`:

```json
{
  "handlers": [
    /* your default handlers */
  ],
  "overrides": [
    {
      "path_pattern": "/home/user/work/*",
      "handlers": [
        {
          "name": "work-notification",
          "type": "desktop",
          "match_rules": null,
          "config": {
            "summary": "Work Project",
            "body": "{{message}}"
          }
        }
      ]
    },
    {
      "path_pattern": "/home/user/personal/secret-project",
      "handlers": [
        /* different handlers for this specific project */
      ]
    }
  ]
}
```

**Behavior:**
- Glob patterns are supported (`*`, `**`, etc.)
- When a pattern matches, override handlers **replace** base handlers completely
- If multiple patterns match, the **last match wins**
- Project-specific `.claude/boopifier.json` and `.opencode/boopifier.json` files still take full precedence

## Available Handlers

| Handler | Description |
|---------|-------------|
| `desktop` | System notifications |
| `sound` | Play audio files |
| `signal` | Signal messenger |
| `webhook` | HTTP webhooks |
| `email` | SMTP email |

Run `boopifier --list-handlers` to see all available types.

## Configuration Examples

### Desktop Notifications

```json
{
  "type": "desktop",
  "config": {
    "summary": "Notification Title",
    "body": "Message with {{variable}} substitution",
    "urgency": "normal",
    "timeout": 5000
  }
}
```

### Slack Webhook

```json
{
  "type": "webhook",
  "config": {
    "url": "{{env.SLACK_WEBHOOK_URL}}",
    "type": "slack",
    "text": "Build {{status}}",
    "channel": "#builds"
  }
}
```

### Signal Messages

```json
{
  "type": "signal",
  "config": {
    "recipient": "+1234567890",
    "message": "Error: {{details}}"
  }
}
```

See [GETTING_STARTED.md](GETTING_STARTED.md) for comprehensive documentation.

## Event Matching

Boopifier receives all fields from hook events and makes them available for both matching rules and template substitution in handler configs. See the [Claude Code hooks documentation](https://code.claude.com/docs/en/hooks) for details on what fields are available for each hook type. OpenCode events are normalized with a `hook_event_name` field (see [event mapping](#opencode-event-mapping)), so the same match rules work for both.

Handlers can match on event fields. Use `null` to match all events.

**Match specific hook events:**
```json
"match_rules": {"hook_event_name": "Notification"}
```

**Match on multiple fields (AND logic):**
```json
"match_rules": {
  "hook_event_name": "Notification",
  "message": "exact message"
}
```

**Regex matching:**
```json
"match_type": "regex",
"match_rules": {
  "hook_event_name": "Notification",
  "message": ".*permission.*"
}
```

**Match multiple events (OR logic):**
```json
"match_rules": {
  "any": [
    {"hook_event_name": "Notification"},
    {"hook_event_name": "Stop"}
  ]
}
```

**Match all events:**
```json
"match_rules": null
```

**Template substitution in config:**
Use `{{field_name}}` to insert event data into handler configs:
```json
"body": "Claude Code: {{message}}"
```

## Development

```bash
# Build
make build

# Run tests
make test

# Lint with clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Generate docs
cargo doc --open
```

See [CLAUDE.md](CLAUDE.md) for detailed development documentation.

## Architecture

```
stdin -> Event Parser -> [OpenCode Normalizer] -> Config Loader -> Event Matcher -> Handler Registry -> Notifications
```

- **Event**: Flexible JSON structure from Claude Code or OpenCode (auto-normalized)
- **Config**: `.claude/boopifier.json` or `.opencode/boopifier.json` with handler definitions
- **Matcher**: Pattern matching to filter events
- **Handlers**: Pluggable notification targets

## Dependencies

Built with blessed.rs-compliant dependencies:
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `clap` - CLI parsing
- `thiserror` / `anyhow` - Error handling
- `notify-rust`, `rodio`, `reqwest`, `lettre` - Notification handlers

## License

Apache-2.0
