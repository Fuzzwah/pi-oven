## Requirements

### Requirement: Tagged-union message envelope

Every wire message SHALL be a JSON object with a top-level `type` field whose value is a string discriminator naming the variant (e.g. `"Hello"`, `"Welcome"`). The Rust `Msg` enum and the TypeScript `Msg` discriminated union SHALL share identical variant names and field shapes byte-for-byte. Adding a new message variant SHALL NOT require changing existing variants.

#### Scenario: Round-trip a Hello frame

- **WHEN** the client serialises `Hello { key: "k", client_version: "0.1.0" }` to JSON and the server deserialises it
- **THEN** the resulting `type` field equals `"Hello"`, `key` equals `"k"`, and `client_version` equals `"0.1.0"`
- **AND** unknown fields (forward-compat) are accepted by the deserialiser without error

#### Scenario: Unknown variant is rejected

- **WHEN** a JSON message with `{"type":"DefinitelyNotAMessage"}` arrives
- **THEN** the receiver MUST treat it as a protocol error, log a structured warning naming the unknown type, and close the connection with WebSocket code `1003` (unsupported data)

### Requirement: Shared-key handshake on first frame

The client SHALL send `Hello { key, client_version }` as the FIRST frame after the WebSocket upgrade completes. The server SHALL validate the key against its configured `shared_key`; on match, the server SHALL reply `Welcome { server_version }` and transition the connection to the active state. On mismatch, the server SHALL send `AuthFailed { reason }` and close the connection with WebSocket close code `4401`.

#### Scenario: Successful handshake

- **WHEN** the client sends `Hello` with a key matching the server's `shared_key`
- **THEN** the server replies with `Welcome { server_version }` within 1 second
- **AND** the connection is logged as authenticated with the client_version

#### Scenario: Wrong key

- **WHEN** the client sends `Hello` with a key that does not match
- **THEN** the server sends `AuthFailed { reason: "invalid_key" }` and closes with code `4401`
- **AND** the client logs the failure and does NOT schedule a reconnect

#### Scenario: Handshake timeout

- **WHEN** the client connects but does not send any frame within 5 seconds
- **THEN** the server closes the connection with code `4408`

#### Scenario: Non-Hello first frame

- **WHEN** the client's first frame is anything other than `Hello`
- **THEN** the server closes the connection with code `4401` and reason `"protocol: expected Hello"`

### Requirement: Origin allowlist on WebSocket upgrade

The server SHALL inspect the `Origin` header on each WebSocket upgrade request and reject the upgrade with HTTP `403` before WebSocket negotiation completes if the Origin is not allowed. A request with no Origin (header absent) or `Origin: null` SHALL be treated as having a `null` Origin and accepted only if config flag `allow_null_origin` is `true` (default `true`). Hosts `localhost` and `127.0.0.1` (any port) SHALL be accepted unconditionally. Other Origins SHALL be accepted only if explicitly listed in `origin_allowlist`.

#### Scenario: Bundled .app sends null Origin and is accepted

- **WHEN** an upgrade request arrives with no `Origin` header (or `Origin: null`) and `allow_null_origin` is `true`
- **THEN** the server completes the WebSocket handshake

#### Scenario: localhost is accepted

- **WHEN** an upgrade request arrives with `Origin: http://localhost:5173`
- **THEN** the server completes the WebSocket handshake regardless of the allowlist contents

#### Scenario: Untrusted Origin is rejected

- **WHEN** an upgrade request arrives with `Origin: https://evil.example.com` and that origin is not in the allowlist
- **THEN** the server responds with HTTP `403` and does NOT upgrade
- **AND** the rejection is logged with the Origin and the remote address

### Requirement: Application-level heartbeat detects dead connections

After a successful handshake, the client SHALL send `Ping { ts_ms }` every 20 seconds. The server SHALL reply `Pong { client_ts_ms, server_ts_ms }` for every received Ping. The client SHALL close its connection if two consecutive pings receive no Pong within 20 seconds each. The server SHALL close any authenticated connection from which it has received no frame for 60 seconds.

#### Scenario: Steady-state heartbeat

- **WHEN** a connection has been authenticated for 60 seconds with normal heartbeating
- **THEN** the client has sent at least 2 `Ping` frames and received corresponding `Pong` frames within 1 second of each
- **AND** neither side closes the connection

#### Scenario: Server stops responding (lid close, network drop)

- **WHEN** the server receives a Ping but its reply does not reach the client (TCP black-hole)
- **THEN** the client closes its connection within 40 seconds of the first unanswered Ping
- **AND** the client schedules a reconnect

#### Scenario: Client goes silent

- **WHEN** the server receives no frame from a previously-authenticated client for 60 seconds
- **THEN** the server closes that connection with code `4001`

### Requirement: Client reconnect with exponential backoff

On any close that is NOT WebSocket code `4401` (auth failed) or `1000` (normal close after explicit user quit), the client SHALL schedule a reconnect attempt after `min(30, 2^attempt)` seconds with ±25% jitter, where `attempt` is the count of consecutive failed connects since the last successful handshake. After a successful handshake, `attempt` SHALL reset to zero. The client SHALL NOT reconnect after close codes `4401` or `4002`.

#### Scenario: Transient network failure recovers

- **WHEN** the connection drops with a network error (close code 1006 abnormal)
- **THEN** the client retries after 1s ±25% (first attempt), 2s ±25% (second), 4s ±25%, …, capped at 30s
- **AND** on a successful handshake the backoff timer resets

#### Scenario: Auth failure does not loop

- **WHEN** the connection closes with code `4401`
- **THEN** the client does NOT schedule a reconnect
- **AND** the client surfaces the auth failure to the UI for operator action

#### Scenario: Replaced does not loop

- **WHEN** the connection closes with code `4002`
- **THEN** the client does NOT schedule a reconnect (a newer client owns the session)

### Requirement: Single connection per shared key

When a new client successfully authenticates while a previously authenticated connection is still open, the server SHALL close the older connection with WebSocket close code `4002` and reason `"replaced"`, then accept the new connection.

#### Scenario: Reopening the .app after a crash

- **WHEN** the .app crashed without sending a close frame and the OS-level socket is still half-open on the server
- **AND** the user relaunches the .app, which authenticates with the same shared key
- **THEN** the server closes the prior connection with `4002 replaced` and the new connection becomes the active one

### Requirement: WebSocket listener bound to configured address

The server SHALL bind its WebSocket listener to the address specified by config key `listen_addr` (default `127.0.0.1:7878`). If the bind fails (port in use, permission denied, address invalid), the server SHALL log a structured error naming `bind` as the failed step and exit with non-zero status, before logging `"ready"`.

#### Scenario: Bind succeeds

- **WHEN** `listen_addr` is `127.0.0.1:7878` and that port is free
- **THEN** the server logs the listener address at `info` level
- **AND** the listener accepts incoming WebSocket upgrades after the `"ready"` log line is emitted

#### Scenario: Port already in use

- **WHEN** another process is bound to `listen_addr`
- **THEN** the server logs an `error` line with `step: "bind"` and exits non-zero
- **AND** the server does NOT emit the `"ready"` log line

### Requirement: Shared key sourced from config file or environment, never auto-generated

The server SHALL resolve `shared_key` from `~/.pi-oven/server.toml` `[net].shared_key` (preferred) or environment variable `PI_OVEN_SHARED_KEY` (fallback). If neither source provides a non-empty value, the server SHALL log a structured error and exit non-zero. The server SHALL NOT auto-generate a shared key.

#### Scenario: Key in config file

- **WHEN** `~/.pi-oven/server.toml` contains `[net]` with `shared_key = "abc"`
- **THEN** that value is used; the env var is NOT consulted

#### Scenario: Key in env var only

- **WHEN** the config file is missing or has no `[net].shared_key` and `PI_OVEN_SHARED_KEY=xyz` is set
- **THEN** that env value is used

#### Scenario: No key anywhere

- **WHEN** neither the config file nor the env var provides a key
- **THEN** the server logs an `error` line naming `shared_key` as missing and exits non-zero

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
