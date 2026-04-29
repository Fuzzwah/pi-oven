## 1. Wire protocol — new message variants

- [x] 1.1 Add `WorkspaceSnapshot`, `Send`, `Abort`, `AgentEvent`, `AgentStatus`, `Resume`, `ReplayBatch`, `ErrorEvent` variants to `packages/pi-oven-server/src/protocol.ts`
- [x] 1.2 Extend `Welcome` message type in `protocol.ts` to include `workspaces: WorkspaceSnapshot[]`
- [x] 1.3 Mirror all new variants in `crates/pi-oven-protocol/src/lib.rs` with matching field names and serde tags
- [x] 1.4 Create golden fixture JSON files in `packages/pi-oven-server/test/fixtures/protocol/` — one file per new variant
- [x] 1.5 Add TypeScript round-trip tests for each new fixture (parse → re-serialise → deep-equal)
- [x] 1.6 Add Rust round-trip tests in `crates/pi-oven-protocol/tests/fixtures.rs` for each new fixture

## 2. Server — NDJSON event log

- [x] 2.1 Create `packages/pi-oven-server/src/workspaces/events/log.ts` — `EventLog` class with `append(event)` (writes NDJSON line with seq + ts) and `replay(last_seq)` (async generator scanning files in order)
- [x] 2.2 Implement seq restoration in `EventLog` constructor — read last line of most recent log file to recover `nextSeq`
- [x] 2.3 Implement log directory + file creation with modes `0700` / `0600` on first write
- [x] 2.4 Implement rotation at 64MB — open new file with incremented rotation suffix, update `currentFile` pointer
- [x] 2.5 Write unit tests for `EventLog`: empty dir (seq starts at 1), existing log (seq restored), append-and-read-back, rotation boundary

## 3. Server — AgentSession

- [x] 3.1 Create `packages/pi-oven-server/src/workspaces/session.ts` — `AgentSession` class wrapping `createAgentSession`
- [x] 3.2 Implement explicit spawn env in `AgentSession` per the plan (gotcha 12): `PATH`, `LANG`, `TZ`, `EDITOR=true`, `TERM=dumb`, `NO_COLOR=1`, `GIT_TERMINAL_PROMPT=0`, `PI_OVEN_WORKSPACE_ID`
- [x] 3.3 Wire pi SDK event listener to seq-stamp each event via `EventLog.append`, then call `onEvent(AgentEvent)` callback
- [x] 3.4 Emit `AgentStatus { status: "running" }` callback when session starts processing; emit `AgentStatus { status: "idle" }` on finish or abort
- [x] 3.5 Implement `AgentSession.queue(text, mode)` — calls pi SDK queue method with the appropriate mode
- [x] 3.6 Implement `AgentSession.abort()` — calls pi SDK abort and ensures `AgentStatus idle` is emitted
- [x] 3.7 Add pi SDK stub (synthetic events on 200ms timer) gated by `PI_OVEN_SDK_STUB=1` env var so server tests work without a real pi install

## 4. Server — WorkspaceManager stub and server routing

- [x] 4.1 Create `packages/pi-oven-server/src/workspaces/manager.ts` — `WorkspaceManager` with `init()` that creates one `AgentSession` (workspace_id=1), `getSession(id)`, and `getSnapshots()` returning `WorkspaceSnapshot[]`
- [x] 4.2 In `WorkspaceManager.init()`, restore seq from the NDJSON log before creating the session
- [x] 4.3 In `packages/pi-oven-server/src/server.ts`, instantiate `WorkspaceManager` after DB init and pass it to the WebSocket message router
- [x] 4.4 In the WebSocket message router, handle `Send` → `manager.getSession(id).queue(text, mode)`, reply `ErrorEvent` if session not found
- [x] 4.5 Handle `Abort` → `manager.getSession(id).abort()`
- [x] 4.6 Handle `Resume` → scan `EventLog.replay(last_seq)`, accumulate buffered in-memory events, send `ReplayBatch`
- [x] 4.7 In `Welcome` handler, populate `workspaces` field from `manager.getSnapshots()`
- [x] 4.8 Wire `AgentSession` event callback to fan-out `AgentEvent` to the connected client; buffer in ring (cap 500) when disconnected

## 5. Server — in-memory event ring buffer

- [x] 5.1 Add `ringBuffer: AgentEvent[]` (max 500) to `WorkspaceManager`
- [x] 5.2 On each event from `AgentSession`, push to ring (evict oldest when over cap) then fan-out if client connected
- [x] 5.3 In `Resume` handler, merge ring buffer events with log replay (deduplicate by seq, return in seq order)

## 6. Client — AppState and event buffer

- [x] 6.1 Add `conversation: RenderedEvent[]`, `scroll_offset: usize`, `follow_mode: bool`, `workspace_status: AgentStatusKind` fields to `AppState` in `crates/pi-oven-ui/src/lib.rs`
- [x] 6.2 Add `last_seq: u64` to `AppState` for replay tracking
- [x] 6.3 Define `RenderedEvent` enum in `crates/pi-oven-ui/src/conversation.rs`: `UserMessage(String)`, `TextDelta(String)`, `ToolCall { name, args_json }`, `ToolResult { output, line_count }`, `StatusChange(String)`, `RawFallback(String)`
- [x] 6.4 Implement `fn append_agent_event(state: &mut AppState, event: AgentEvent)` — parse the inner `event` JSON, push the appropriate `RenderedEvent` variant, accumulate text deltas into the current assistant bubble
- [x] 6.5 Implement tab expansion in `append_agent_event` — expand `\t` to 8-column stops before storing text content

## 7. Client — conversation widget rendering

- [x] 7.1 Implement `render_conversation(frame, area, state)` in `crates/pi-oven-ui/src/conversation.rs` — iterate `state.conversation`, render each `RenderedEvent` variant with appropriate style
- [x] 7.2 Render `UserMessage` with `> ` prefix and accent colour
- [x] 7.3 Render `TextDelta` accumulated text as plain assistant lines
- [x] 7.4 Render `ToolCall` as `▶ <name>` in muted style (args collapsed)
- [x] 7.5 Render `ToolResult` — show up to 10 lines; if `line_count > 10`, append `… N more lines` indicator
- [x] 7.6 Render `RawFallback` as a single muted line with the raw JSON
- [x] 7.7 Apply `scroll_offset` when computing which lines to paint (viewport window into the full rendered line list)
- [x] 7.8 Wire `render_conversation` into the main `render` function in `lib.rs`

## 8. Client — scroll-pinning and follow mode

- [x] 8.1 In the wgpu key handler, map `ArrowUp` / `PageUp` in conversation context to decrement `scroll_offset` and set `follow_mode = false`
- [x] 8.2 Map `ArrowDown` / `PageDown` to increment `scroll_offset`; when viewport reaches bottom, set `follow_mode = true`
- [x] 8.3 In `render_conversation`, when `follow_mode` is true, compute the scroll offset needed to show the last line and update `state.scroll_offset` before painting

## 9. Client — Send and Abort dispatch

- [x] 9.1 In `wgpu_main::handle_key`, match `NamedKey::Enter` (no modifiers) — take `state.editor.text()`, push a `UserMessage` to `state.conversation`, send `Send { workspace_id: 1, text, queue_mode: "steer" }` over the WebSocket, clear the editor
- [x] 9.2 Match `Alt+Enter` — same as Enter but `queue_mode: "followup"`
- [x] 9.3 Match `NamedKey::Escape` during `running` status — send `Abort { workspace_id: 1 }`
- [x] 9.4 Mirror all three actions in the crossterm key handler (`Enter`, `Alt+Enter`, `Escape`)

## 10. Client — Welcome handling and Resume on connect

- [x] 10.1 In the net layer (`crates/pi-oven-net/src/`), after receiving `Welcome`, send `Resume { workspace_id: 1, last_seq: state.last_seq }` for each workspace in `workspaces`
- [x] 10.2 Handle `ReplayBatch` — call `append_agent_event` for each event in `events`, update `state.last_seq` to `latest_seq`
- [x] 10.3 Handle `AgentEvent` — call `append_agent_event`, update `state.last_seq`
- [x] 10.4 Handle `AgentStatus` — update `state.workspace_status`, trigger redraw

## 11. End-to-end smoke test

- [x] 11.1 Start server with `PI_OVEN_SDK_STUB=1`; verify `Welcome` carries `workspaces: [{ workspace_id: 1, status: "idle" }]`
- [x] 11.2 Connect client; verify `Resume` is sent and `ReplayBatch` is received before any `AgentEvent`
- [x] 11.3 Type a message in the input bar and press Enter; verify `Send` frame is sent, `UserMessage` appears in the conversation pane, `AgentStatus running` arrives, synthetic events render, `AgentStatus idle` arrives
- [x] 11.4 Press Escape mid-stream; verify `Abort` is sent and session returns to idle
- [x] 11.5 Kill the client mid-stream; relaunch; verify conversation replays from seq 1 and rendering is correct
- [x] 11.6 Verify NDJSON log file at `~/.pi-oven/events/1/` contains one line per event with correct `seq`, `ts`, and `event` fields
