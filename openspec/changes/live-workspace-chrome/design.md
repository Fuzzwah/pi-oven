## Context

The client already receives `Welcome { workspaces: WorkspaceSnapshot[] }` and `AgentStatus { workspace_id, status }` messages and routes them through `process_msg`. Currently `Welcome` only triggers a `Resume` request; `AgentStatus` only updates `workspace_status: AgentStatusKind`. Neither touches the tab strip or header. `AppState::default()` still populates `tabs` with four hardcoded mock entries.

`WorkspaceSnapshot` carries `{ workspace_id: u64, status: String }`. That is the full picture the server currently exposes — no project name, branch, or model yet.

## Goals / Non-Goals

**Goals:**
- Tab strip reflects actual workspaces from the server after `Welcome`
- `TabStatus` tracks real `running`/`idle` states from `AgentStatus`
- Active workspace is tracked so header and status bar can key off it
- `AppState::default()` produces an empty / connecting state rather than mock data

**Non-Goals:**
- Populating model name, branch, or PR from the server (server doesn't send these yet — leave as `"–"` placeholders)
- Multi-workspace tab switching / keyboard navigation (separate slice)
- Persisting active workspace across reconnects

## Decisions

### D1 — Build `TabCell` from `WorkspaceSnapshot` in `process_msg`

On `Welcome`, clear `state.tabs` and rebuild from `workspaces`. Set `active_workspace_id` to the first workspace's id (or 0 if empty). Each `TabCell` gets `project = format!("ws-{}", workspace_id)` and `trigger = format!("ws-{}", workspace_id)` as a minimal placeholder until the server sends richer metadata.

**Alternative considered:** Keeping mock tabs until a richer snapshot arrives. Rejected — shows stale data after real connection.

### D2 — `TabStatus` driven by `AgentStatus` only

On `AgentStatus { workspace_id, status }`, find the matching `TabCell` in `state.tabs` and set its `status` to `TabStatus::Active` (running) or `TabStatus::Idle`. No `Attention` variant in this slice.

The existing `workspace_status: AgentStatusKind` field is kept for the input bar's Escape-to-abort guard; `TabStatus` is a separate parallel field on each cell.

### D3 — `active_workspace_id: u64` added to `AppState`

A single `u64` field, defaulting to `0` (sentinel for "no workspace yet"). Set to first workspace id on `Welcome`. Used by header and status bar rendering to pick which cell's data to display. No UI to change it this slice.

### D4 — `AppState::default()` becomes a blank/connecting state

Remove all four hardcoded mock `TabCell` entries. Set `header.title` to `"Connecting…"` and status fields to `"–"`. After `Welcome` the real data populates; the blank state is only visible during the brief connection window.

**Alternative:** Keep mocks, overlay real data. Rejected — confusing to see mock project names flash then disappear.

## Risks / Trade-offs

- **Single active workspace** — UI only ever highlights one workspace. Fine for now; multi-tab selection is a future slice.
- **Sparse WorkspaceSnapshot** — server only sends `workspace_id` + `status`. Project/branch/model shown as `"–"` until server is extended. Acceptable: the layout is correct and placeholders are visually distinct from real data.
- **Reconnect resets tabs** — on reconnect the new `Welcome` replaces `state.tabs` entirely. Any mid-reconnect state (e.g. a tab the user mentally selected) is lost. Acceptable for this slice.
