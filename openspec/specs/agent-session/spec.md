## Requirements

### Requirement: AgentSession wraps one pi SDK session per workspace

The server SHALL create an `AgentSession` instance for each active workspace by calling `createAgentSession` from `@mariozechner/pi-coding-agent`. Each `AgentSession` SHALL own a single pi session identified by a `workspace_id`. The `AgentSession` SHALL expose `queue(text: string, mode: 'steer' | 'followup'): void` and `abort(): void` to callers.

#### Scenario: Session created at startup for the hardcoded workspace

- **WHEN** the server boots and the WebSocket listener is ready
- **THEN** one `AgentSession` is created with `workspace_id = 1` and the session is in the `idle` state
- **AND** the session is accessible to the message router for the lifetime of the server

#### Scenario: queue delivers a message to the pi session

- **WHEN** `session.queue("hello", "steer")` is called
- **THEN** the underlying pi SDK session receives the message with steer semantics
- **AND** the session transitions to `running` state

#### Scenario: abort cancels the current turn

- **WHEN** `session.abort()` is called while the session is in `running` state
- **THEN** the pi SDK session's abort method is called
- **AND** an `AgentStatus { workspace_id, status: "idle" }` event is emitted after the abort completes

### Requirement: Monotonic seq stamped on every event before log write

The server SHALL maintain a per-workspace monotonic integer counter `seq` starting at 1. Before writing any pi event to the NDJSON log, the server SHALL atomically increment `seq` and attach it to the event record. `seq` SHALL be restored from the last line of the NDJSON log on server boot.

#### Scenario: seq is monotonically increasing

- **WHEN** a series of pi events arrive from the session
- **THEN** each `AgentEvent` emitted has a `seq` strictly greater than the previous one for the same workspace
- **AND** no two events share the same `seq` value for a given `workspace_id`

#### Scenario: seq restores from log on boot

- **WHEN** the server boots and the NDJSON log for workspace 1 exists with last entry `{ "seq": 42, ... }`
- **THEN** the in-memory `seq` counter starts at 43
- **AND** the first new event receives `seq = 43`

#### Scenario: seq starts at 1 for a fresh workspace

- **WHEN** the NDJSON log for a workspace does not exist or is empty
- **THEN** the first event receives `seq = 1`

### Requirement: Append-only NDJSON event log written before fan-out

The server SHALL write each seq-stamped event as a single NDJSON line to `<data_dir>/events/<workspace_id>/<created_at_ms>-0.ndjson` before fanning it out to connected clients. Each line SHALL be a valid JSON object with at minimum `{ "seq": N, "ts": <unix_ms>, "event": <pi event object> }`. The write MUST complete (fsync not required; `write` syscall returning success is sufficient) before the event is sent over the WebSocket.

#### Scenario: Event is in log before client receives it

- **WHEN** a pi event arrives and is fanned out to the client
- **THEN** the NDJSON log line with the matching `seq` exists on disk before the WebSocket frame is sent
- **AND** the log line parses as valid JSON

#### Scenario: Log directory is created if missing

- **WHEN** the server handles the first event for a workspace and `<data_dir>/events/<workspace_id>/` does not exist
- **THEN** the directory is created with mode `0700` before the first write
- **AND** the log file is created with mode `0600`

#### Scenario: Log file rotation at 64 MB

- **WHEN** the current log file reaches 64 MB
- **THEN** the server closes the current file and opens a new one named `<created_at_ms>-1.ndjson` (incrementing the rotation suffix)
- **AND** subsequent events are written to the new file

### Requirement: Fan-out AgentEvent to connected client

The server SHALL, after writing an event to the NDJSON log, send `AgentEvent { workspace_id, seq, event }` to the currently authenticated WebSocket client, if one is connected. If no client is connected, the server SHALL buffer the event in memory (ring of last 500 events) and deliver it on the next `Resume` request.

#### Scenario: Client connected — event delivered immediately

- **WHEN** a pi event arrives and a client is connected and authenticated
- **THEN** the server sends `AgentEvent { workspace_id, seq, event }` to the client within one event-loop tick after the log write
- **AND** events are delivered in seq order with no gaps

#### Scenario: Client disconnected — event buffered

- **WHEN** a pi event arrives and no client is connected
- **THEN** the event is added to the in-memory ring buffer (capped at 500 events)
- **AND** when a client later connects and sends `Resume`, the buffered events are included in `ReplayBatch`

#### Scenario: Ring buffer overflow drops oldest events

- **WHEN** more than 500 events accumulate while no client is connected
- **THEN** the oldest events are evicted from the ring buffer
- **AND** a client reconnecting with `last_seq` before the oldest buffered event receives a full replay from the NDJSON log instead

### Requirement: Replay from NDJSON log on Resume

The server SHALL handle `Resume { workspace_id, last_seq }` by scanning the NDJSON log for all events with `seq > last_seq` and returning them in a `ReplayBatch { workspace_id, events, latest_seq }`. The server SHALL buffer any new events that arrive during the scan and include them in the batch before resuming normal fan-out.

#### Scenario: Client reconnects and receives missed events

- **WHEN** a client sends `Resume { workspace_id: 1, last_seq: 10 }` and the NDJSON log has entries with seq 1 through 25
- **THEN** the server replies with `ReplayBatch { workspace_id: 1, events: [<seq 11..25>], latest_seq: 25 }`
- **AND** subsequent `AgentEvent` messages continue from seq 26

#### Scenario: Client is already up to date

- **WHEN** a client sends `Resume { workspace_id: 1, last_seq: 25 }` and the latest seq in the log is 25
- **THEN** the server replies with `ReplayBatch { workspace_id: 1, events: [], latest_seq: 25 }`

#### Scenario: Resume for unknown workspace is rejected

- **WHEN** a client sends `Resume { workspace_id: 999, last_seq: 0 }` and no such workspace exists
- **THEN** the server sends `ErrorEvent { reason: "unknown_workspace" }` and does NOT close the connection

### Requirement: AgentStatus emitted on session state changes

The server SHALL emit `AgentStatus { workspace_id, status: "running" | "idle" }` to the connected client whenever the session transitions between states. `"running"` is emitted when the pi session begins processing; `"idle"` is emitted when it finishes or is aborted.

#### Scenario: Status changes are delivered in order relative to AgentEvents

- **WHEN** a session transitions from idle to running, emits several events, then returns to idle
- **THEN** the client receives `AgentStatus running`, then the `AgentEvent` sequence, then `AgentStatus idle`
- **AND** the `seq` values of interleaved `AgentEvent` messages are contiguous
