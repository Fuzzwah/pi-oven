## Context

The server boots, the WebSocket handshake completes, and the client renders four static panes. Nothing real happens yet. This change completes the Slice 1 skeleton by adding the thinnest possible path from "user types a message" to "pi responds and the conversation renders". No workspace picker, no tabs, no project management — just one hardcoded session proving the full loop works.

The existing architecture already provides the scaffolding:
- `WorkspaceManager` and `AgentSession` are referenced in `docs/claude_plan.md` and the repo layout is specified, but the files don't exist yet
- `protocol.ts` / `codec.rs` have handshake messages only — no session messages
- The conversation widget renders placeholder text only
- The input bar captures keystrokes but has no dispatch path to the server

## Goals / Non-Goals

**Goals:**
- One pi SDK session alive per server boot (hardcoded; no picker UI)
- `Send` / `Abort` messages routed from client input bar to the session
- `AgentEvent` fan-out from the session to the connected client, seq-stamped
- Append-only NDJSON event log durably written before fan-out
- `Resume` / `ReplayBatch` replay from the NDJSON log on reconnect
- Conversation pane renders streamed events (text deltas, tool calls, tool results, status)
- TUI baseline: scroll-pinning, tab expansion, input wrap, continuation indent

**Non-Goals:**
- Multiple workspaces or tabs (Slice 2)
- Workspace picker flow (Slice 4)
- Image attachments (Slice 3)
- Session persistence across server restart (Slice 2 / gotcha 3 from the plan)
- Any project management messages (`AddProject`, etc.)

## Decisions

### D1: Single hardcoded workspace at boot, no WorkspaceManager abstraction yet

The full `WorkspaceManager` (eager re-attach, orphan cleanup, tab ordering) is Slice 2 scope. Introducing the abstraction now without multi-workspace use creates dead code and premature design surface. Instead, `server.ts` calls `AgentSession` directly at boot with a hardcoded `workspace_id = 1`; `manager.ts` is a thin shim that will be replaced in Slice 2.

**Alternative considered:** Full `WorkspaceManager` now. Rejected — too much surface for a prototype slice, and the eager-reattach path requires pi SDK resume to be validated first (the SDK spike).

### D2: seq assigned server-side at write time, before fan-out

`seq` is a per-workspace monotonic integer. The server increments it atomically, writes the event to the NDJSON log, then fans out `AgentEvent { workspace_id, seq, event }` to the connected client. This ordering guarantees: if the server crashes before fan-out, the event is in the log and will be replayed on `Resume`. If the server crashes after fan-out but before log write — impossible by construction.

`seq` is kept in memory (starting from the log's last line on boot) and not persisted to SQLite, since the NDJSON log is the authoritative store.

### D3: NDJSON log path and rotation

Per the plan (gotcha 6): `~/.pi-oven/events/<workspace_id>/<created_at>-<rot>.ndjson`, rotate at 64MB. For Slice 1 (single session, short-lived), rotation will never trigger, but the writer must be structured to support it. The reader for `Resume` scans files in sort order and skips `seq <= last_seq`.

**Alternative considered:** Write events to SQLite `events` table. Rejected — pi events can be large (long tool outputs); SQLite text storage is fine for metadata but the plan explicitly chose NDJSON for event durability. Keeping this consistent with the plan avoids a migration later.

### D4: AgentEvent payload is a raw pi SDK event passthrough

The server does not interpret or transform pi events — it wraps them as `AgentEvent { workspace_id, seq, event: <pi event JSON> }`. The client renders what it receives. This avoids defining a server-side normalisation layer before we know the full shape of pi's event stream. The conversation widget is the only consumer; it can handle unknown event types gracefully (render as raw text fallback).

### D5: Conversation pane scroll model

Two modes: **follow** (default) and **pinned**. In follow mode, each new event scrolls the viewport to show the latest content. In pinned mode (user has scrolled up), new events append without moving the viewport. Scrolling to the bottom re-enters follow mode. This is the "scroll-position pinning during streaming" baseline requirement from the plan.

The viewport offset is stored in `AppState` as a `scroll_offset: usize` (lines from top). The conversation widget reports its rendered line count after each draw; the app shell compares this to the viewport height to decide whether follow mode should scroll.

### D6: TUI baseline features land atomically with this slice

Scroll-pinning, tab expansion, input wrap, and continuation-row indent are called out in the plan as "bake in from the start, not polish". They are simplest to implement alongside the first real rendering rather than retrofitted. Tab expansion is a filter on incoming event text before storing in the buffer (expand `\t` to spaces to reach the next 8-column boundary). Input wrap and continuation indent are purely in the `render_input` widget, already tested.

### D7: Wire protocol golden fixtures for new messages

New message variants (`Send`, `Abort`, `AgentEvent`, `AgentStatus`, `Resume`, `ReplayBatch`, `WorkspaceSnapshot`) must have golden JSON fixture files in `test/fixtures/protocol/` — one file per variant. Both the TypeScript parse test and the Rust round-trip test must pass for each new fixture. This is the protocol-drift guard from gotcha 15.

## Risks / Trade-offs

**[Risk] pi SDK `createAgentSession` API shape is unknown** → The SDK spike (Slice 0 prerequisite) should have produced `docs/pi-sdk-notes.md`. If it hasn't been done yet, `session.ts` should use a local stub that emits synthetic events on a timer, allowing all other work to proceed in parallel. The stub is removed when the real SDK is wired.

**[Risk] Large pi events exceed the 16MB WebSocket frame cap** → The plan (gotcha 10) specifies chunking via `LargeEventChunk`. For Slice 1, the risk is low (no long file-cat tool calls yet). Add a TODO comment at the fan-out point noting where chunking will go; do not implement yet.

**[Risk] NDJSON log grows unboundedly in long sessions** → Rotation is specified at 64MB. For Slice 1, a single session won't hit this. The writer must be structured so rotation can be bolted on without rewiring the reader — use a `currentFile` pointer in `EventLog` that the writer can swap atomically.

**[Risk] Reconnect replay races with live fan-out** → Between the client sending `Resume` and the server completing the log scan, new events arrive. Solution: server starts the log scan, captures `head_seq` at scan start, buffers any events with `seq > head_seq` during scan, includes them in `ReplayBatch`, then resumes normal fan-out. This is a single-threaded Node process so no actual concurrency issue — the event loop handles one message at a time.

## Migration Plan

No schema migration needed for this slice (event logs are filesystem-only). The new message variants in `protocol.ts` and `codec.rs` are additive — existing handshake tests continue to pass. Deploy order: server first, then client (client will reconnect cleanly on server restart).

## Open Questions

- **pi SDK `session.queue` signature**: Does it accept a `queue_mode` parameter (steer vs followup) directly, or does the caller manage this? The SDK notes should answer this; if not, default to treating all sends as steer mode for Slice 1.
- **pi SDK event shapes**: What event types does pi emit and what are their JSON field names? The conversation widget needs to handle at minimum: text delta, tool call start, tool call result, assistant status. Unknown types fall back to a raw JSON render.
