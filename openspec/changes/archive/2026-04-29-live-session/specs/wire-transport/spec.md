## ADDED Requirements

### Requirement: WorkspaceSnapshot delivered on Welcome

The server SHALL include a `workspaces` field in the `Welcome` message containing a `WorkspaceSnapshot[]` — one entry per active workspace known to the server at handshake time. Each snapshot SHALL carry at minimum `{ workspace_id, status: "running" | "idle" }`. The client SHALL initialise its workspace state from this snapshot before requesting replay.

#### Scenario: Server with one active workspace includes it in Welcome

- **WHEN** the server has one active workspace (id=1, status="idle") and a client authenticates
- **THEN** the `Welcome` message contains `workspaces: [{ workspace_id: 1, status: "idle" }]`
- **AND** the client can immediately send `Resume { workspace_id: 1, last_seq: 0 }` to populate its conversation buffer

#### Scenario: Server with no active workspaces sends empty array

- **WHEN** the server has no active workspaces and a client authenticates
- **THEN** the `Welcome` message contains `workspaces: []`

### Requirement: Send message routes user input to the active session

The client SHALL send `Send { workspace_id, text, queue_mode: "steer" | "followup" }` to deliver a message to an active pi session. The server SHALL route this to the matching `AgentSession.queue(text, mode)` call. If no session exists for the given `workspace_id`, the server SHALL reply with `ErrorEvent { reason: "unknown_workspace" }`.

#### Scenario: Send with steer mode queues a steering message

- **WHEN** the client sends `Send { workspace_id: 1, text: "hello", queue_mode: "steer" }`
- **THEN** the server calls `session.queue("hello", "steer")`
- **AND** the session transitions to `running` and emits an `AgentStatus { status: "running" }` event

#### Scenario: Send to unknown workspace returns error

- **WHEN** the client sends `Send { workspace_id: 999, text: "hi", queue_mode: "steer" }`
- **THEN** the server sends `ErrorEvent { workspace_id: 999, reason: "unknown_workspace" }`
- **AND** the connection remains open

### Requirement: Abort message cancels the current agent turn

The client SHALL send `Abort { workspace_id }` to cancel the pi session's current turn. The server SHALL call `session.abort()` on the matching session. If the session is already idle, the server SHALL send `AgentStatus { workspace_id, status: "idle" }` without error.

#### Scenario: Abort while running cancels the turn

- **WHEN** the client sends `Abort { workspace_id: 1 }` and the session is in `running` state
- **THEN** the server calls `session.abort()` and the pi session stops generating
- **AND** the server sends `AgentStatus { workspace_id: 1, status: "idle" }` once the abort completes

#### Scenario: Abort while idle is a no-op

- **WHEN** the client sends `Abort { workspace_id: 1 }` and the session is in `idle` state
- **THEN** the server sends `AgentStatus { workspace_id: 1, status: "idle" }` without calling abort

### Requirement: AgentEvent carries seq-stamped pi event to client

The server SHALL send `AgentEvent { workspace_id, seq, event }` for every pi SDK event. `seq` SHALL be a per-workspace monotonic integer (see agent-session spec). `event` SHALL be the raw pi event object. Clients SHALL accept and store all `AgentEvent` messages, rendering unknown event types as a raw fallback.

#### Scenario: AgentEvent round-trips through the wire codec

- **WHEN** the server serialises `AgentEvent { workspace_id: 1, seq: 7, event: { type: "text_delta", text: "hi" } }` and the client deserialises it
- **THEN** the client has `workspace_id = 1`, `seq = 7`, and `event.type = "text_delta"`

### Requirement: AgentStatus notifies client of session state changes

The server SHALL send `AgentStatus { workspace_id, status: "running" | "idle" }` when the pi session transitions state. The client SHALL use this to update UI indicators (e.g. a spinner in the tab or status line).

#### Scenario: AgentStatus round-trips through the wire codec

- **WHEN** the server serialises `AgentStatus { workspace_id: 1, status: "running" }` and the client deserialises it
- **THEN** the client has `workspace_id = 1` and `status = "running"`

### Requirement: Resume and ReplayBatch enable reconnect without data loss

The client SHALL send `Resume { workspace_id, last_seq }` after authenticating to request all events it missed since `last_seq`. The server SHALL reply with `ReplayBatch { workspace_id, events, latest_seq }` containing all stored events with `seq > last_seq` for that workspace.

#### Scenario: ReplayBatch round-trips through the wire codec

- **WHEN** the server serialises `ReplayBatch { workspace_id: 1, events: [{ seq: 11, ts: 1000, event: {} }], latest_seq: 11 }` and the client deserialises it
- **THEN** the client has `workspace_id = 1`, one event with `seq = 11`, and `latest_seq = 11`

#### Scenario: Client sends Resume immediately after Welcome

- **WHEN** the client receives `Welcome` with `workspaces: [{ workspace_id: 1 }]`
- **THEN** the client sends `Resume { workspace_id: 1, last_seq: <last known seq or 0> }` before sending any other message
- **AND** the server replies with `ReplayBatch` before delivering new `AgentEvent` messages

### Requirement: Golden fixtures for all new message variants

Each new message variant SHALL have a corresponding JSON fixture file in `packages/pi-oven-server/test/fixtures/protocol/`. The TypeScript parse test and Rust round-trip test SHALL both pass for every fixture. A new message variant MUST NOT be merged without a corresponding fixture.

#### Scenario: New message has a fixture file

- **WHEN** a developer adds a new `Msg` variant to `protocol.ts` and `codec.rs`
- **THEN** a fixture file at `test/fixtures/protocol/<VariantName>.json` exists and both TS and Rust round-trip tests pass for it
