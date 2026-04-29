## Why

The WebSocket handshake and server boot are in place but the app still renders static scaffolding — no real pi session runs, no messages flow. This change wires a single hardcoded workspace end-to-end: the user types in the input bar, the server queues the message to a live pi SDK session, events stream back and render in the conversation pane, and every event is durably written to the NDJSON event log so reconnects replay seamlessly.

## What Changes

- Server creates one pi SDK session at startup (hardcoded workspace, no picker flow yet)
- New `AgentSession` module wraps `createAgentSession`, assigns monotonic `seq` to every pi event, and appends them to a per-workspace NDJSON log before fan-out
- `WorkspaceManager` stub: single workspace lifecycle (create, fan-out, replay) — multi-workspace and tabs land in Slice 2
- New wire messages: `WorkspaceSnapshot`, `Send`, `Abort`, `AgentEvent`, `AgentStatus`, `Resume`, `ReplayBatch` added to the shared `Msg` enum
- Client conversation pane renders streamed `AgentEvent` payloads (text deltas, tool calls, tool results, status lines)
- Input bar dispatches `Send` on Enter / `Abort` on Escape
- On reconnect, client sends `Resume { workspace_id, last_seq }` and conversation pane replays from the NDJSON log
- TUI baseline behaviour baked in: scroll-pinning during streaming, tab expansion to 8-column stops, input text wraps inside the box, continuation-row indent

## Capabilities

### New Capabilities

- `agent-session`: Server-side pi SDK session adapter — session lifecycle, seq-stamped event fan-out, append-only NDJSON event log, replay-from-log on `Resume`
- `conversation-pane`: Client-side widget that renders a sequence of `AgentEvent` payloads as a scrollable conversation with streaming, scroll-pinning, tab expansion, and inline code blocks

### Modified Capabilities

- `wire-transport`: Add active-session messages — `WorkspaceSnapshot`, `Send`, `Abort`, `AgentEvent`, `AgentStatus`, `Resume`, `ReplayBatch` — to the shared `Msg` enum and golden fixtures

## Impact

- `packages/pi-oven-server/src/workspaces/` — new `manager.ts`, `session.ts`
- `packages/pi-oven-server/src/workspaces/events/` — new `log.ts` (NDJSON writer/reader)
- `packages/pi-oven-server/src/server.ts` — route new message types to workspace manager
- `packages/pi-oven-server/src/protocol.ts` — new message variants
- `crates/pi-oven-protocol/src/lib.rs` — matching Rust `Msg` variants
- `crates/pi-oven-ui/src/conversation.rs` — new rendering logic
- `crates/pi-oven-ui/src/lib.rs` — `AppState` gains conversation event buffer and workspace metadata
- `crates/pi-oven/src/` (wgpu + crossterm mains) — Enter/Escape dispatch to Send/Abort
- `packages/pi-oven-server/test/fixtures/protocol/` — golden fixtures for new message types
- New dependency: `@mariozechner/pi-coding-agent` (already declared in `package.json`, first real use)
