## Why

The UI chrome (tab strip, conversation header, status bar) still shows hard-coded placeholder strings from `AppState::default()`. The server already sends `WorkspaceSnapshot[]` in `Welcome` and `AgentStatus` updates on every state change — the data is arriving, but the client ignores it and never updates the display.

## What Changes

- On `Welcome`, populate `AppState.tabs` from the received `WorkspaceSnapshot[]` list, replacing the hardcoded mock tabs.
- On `AgentStatus`, update the matching tab's status indicator and the header/status bar if that workspace is the active one.
- Add an `active_workspace_id: u64` field to `AppState` to track which workspace is focused; default to the first workspace received in `Welcome`.
- Wire `AppState.header.title` to show the active workspace's trigger string (or a sensible fallback) rather than the hardcoded placeholder.
- Wire `AppState.status.model`, `AppState.status.ctx`, `AppState.status.branch`, `AppState.status.pr` to values derivable from `WorkspaceSnapshot` — initially model and branch are still placeholders, but status/idle indicators are real.
- `TabStatus::Active` / `Idle` / `Attention` driven by `AgentStatus` messages rather than hardcoded.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `client-ui`: `AppState` gains `active_workspace_id`; tab cells and header are updated from server messages rather than static defaults.
- `wire-transport`: Client-side handling of `Welcome` and `AgentStatus` extended to update chrome state in addition to the conversation buffer.

## Impact

- `crates/pi-oven-ui/src/lib.rs`: `AppState` — new `active_workspace_id` field, `tabs` built from `WorkspaceSnapshot`.
- `crates/pi-oven/src/main.rs`: `process_msg` — `Welcome` handler builds tabs; `AgentStatus` handler updates tab status and triggers header refresh.
- Layout tests updated for new `AppState::default()` tab structure.
- No server changes required.
