## ADDED Requirements

### Requirement: Server translates pi SDK events into protocol events before logging

The server SHALL subscribe to `AgentSessionEvent` from the pi SDK and translate each event into a protocol `event` payload before appending it to the NDJSON log and fanning it out. Translation SHALL apply the following deterministic mapping; any SDK event type not listed SHALL be dropped (not logged, not fanned out):

| SDK event type                                          | Translated protocol event                                                                                            |
|---------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------|
| `message_update` whose inner assistant event is `text_delta` | `{ type: "text_delta", text: <delta string> }`                                                                       |
| `tool_execution_start`                                  | `{ type: "tool_call", tool_name: <toolName>, args: <args> }`                                                          |
| `tool_execution_end`                                    | `{ type: "tool_result", tool_name: <toolName>, output: <result stringified>, exit_code: <isError ? 1 : 0> }`         |

`turn_start`, `turn_end`, `message_start`, `message_end`, `queue_update`, `compaction_start`, `compaction_end`, `auto_retry_start`, `auto_retry_end`, `session_info_changed` SHALL be dropped. The translator SHALL be a pure function with no I/O so it is unit-testable in isolation. The translator's output (and only its output) SHALL be what the existing "Monotonic seq stamped on every event before log write" and "Append-only NDJSON event log written before fan-out" requirements operate on.

#### Scenario: text_delta is translated and forwarded

- **WHEN** the SDK emits a `message_update` event whose inner `assistantMessageEvent.type` is `text_delta` with `delta = "hello"`
- **THEN** the server appends `{ "seq": N, "ts": ..., "event": { "type": "text_delta", "text": "hello" } }` to the NDJSON log
- **AND** the same event payload is sent to the connected client as `AgentEvent`

#### Scenario: tool execution emits a paired call/result

- **WHEN** the SDK emits `tool_execution_start { toolCallId: "t1", toolName: "bash", args: { command: "ls" } }` followed by `tool_execution_end { toolCallId: "t1", toolName: "bash", result: "file1\nfile2", isError: false }`
- **THEN** the log contains, in order, `{ "type": "tool_call", "tool_name": "bash", "args": { "command": "ls" } }` and `{ "type": "tool_result", "tool_name": "bash", "output": "file1\nfile2", "exit_code": 0 }`

#### Scenario: failed tool execution sets exit_code to 1

- **WHEN** the SDK emits `tool_execution_end { toolCallId: "t1", toolName: "bash", result: "permission denied", isError: true }`
- **THEN** the translated event is `{ "type": "tool_result", "tool_name": "bash", "output": "permission denied", "exit_code": 1 }`

#### Scenario: dropped event types do not appear in the log or on the wire

- **WHEN** the SDK emits `turn_start`, `message_start`, `message_end`, `turn_end`, `queue_update`, or any other unmapped event type
- **THEN** no NDJSON line is appended for that event
- **AND** no `AgentEvent` frame is sent to the client for that event
- **AND** the seq counter is not advanced

### Requirement: AgentStatus transitions are derived from SDK lifecycle events

The server SHALL emit `AgentStatus { workspace_id, status: "running" }` when the SDK emits `agent_start`, and `AgentStatus { workspace_id, status: "idle" }` when the SDK emits `agent_end`. `AgentStatus` SHALL also be emitted with `status: "idle"` after a successful `abort()` even if no `agent_end` arrives. Lifecycle events themselves SHALL NOT be appended to the NDJSON log (they are status-only signalling, not part of the conversation transcript).

#### Scenario: agent_start triggers running

- **WHEN** the SDK emits an `agent_start` event for workspace 1
- **THEN** the server sends `AgentStatus { workspace_id: 1, status: "running" }` to the connected client

#### Scenario: agent_end triggers idle

- **WHEN** the SDK emits an `agent_end` event for workspace 1
- **THEN** the server sends `AgentStatus { workspace_id: 1, status: "idle" }` to the connected client

#### Scenario: explicit abort guarantees idle even if SDK does not emit agent_end

- **WHEN** the client sends `Abort { workspace_id: 1 }` and the SDK's `abort()` resolves without subsequently emitting `agent_end`
- **THEN** the server sends `AgentStatus { workspace_id: 1, status: "idle" }` to the connected client

### Requirement: Server initialises pi SDK eagerly at boot and fails fast on missing usable model

When `PI_OVEN_SDK_STUB` is not set, the server SHALL invoke `createAgentSession({ cwd, agentDir })` from `WorkspaceManager.init()` (i.e. before the WebSocket listener binds). If `createAgentSession` returns with `model === undefined`, the server SHALL fail boot with an error message that includes `result.modelFallbackMessage`. The `cwd` for the hardcoded workspace 1 SHALL be `<data_dir>/workspaces/1` and SHALL be created with mode `0700` if it does not already exist. Before the first `createAgentSession` call, the server SHALL set `LANG`, `TZ`, `EDITOR`, `TERM`, `NO_COLOR`, `GIT_TERMINAL_PROMPT`, and `PI_OVEN_WORKSPACE_ID` on `process.env` to the values defined in [docs/claude_plan.md](../../../docs/claude_plan.md) §12, but SHALL NOT overwrite values already present in `process.env`.

#### Scenario: missing usable model fails boot fast

- **WHEN** the server starts without `PI_OVEN_SDK_STUB=1` and `~/.pi/agent/auth.json` has no credentials for any model the SDK can resolve
- **THEN** boot fails before the WebSocket listener binds
- **AND** the structured log emits an error line with `step: "workspace_manager"` and a message including the SDK's `modelFallbackMessage`

#### Scenario: workspace cwd is created if missing

- **WHEN** the server starts and `<data_dir>/workspaces/1` does not exist
- **THEN** the directory is created with mode `0700`
- **AND** the SDK is given that path as `cwd`

#### Scenario: process env defaults do not clobber user-provided values

- **WHEN** the user starts the server with `LANG=de_DE.UTF-8` already set in the environment
- **THEN** `process.env.LANG` remains `"de_DE.UTF-8"` after `applyChildProcessEnv` runs
- **AND** the other defaults (`EDITOR=true`, `TERM=dumb`, `NO_COLOR=1`, `GIT_TERMINAL_PROMPT=0`, `PI_OVEN_WORKSPACE_ID=1`) are applied because they were unset

### Requirement: PI_OVEN_SDK_STUB short-circuits the real SDK path

When the environment variable `PI_OVEN_SDK_STUB` equals `"1"`, the server SHALL use the existing synthetic-event stub path and SHALL NOT call `createAgentSession`. This SHALL allow tests and offline development to run without real auth.

#### Scenario: stub path emits canned events without calling the SDK

- **WHEN** the server starts with `PI_OVEN_SDK_STUB=1` and the client sends `Send { workspace_id: 1, text: "hi", queue_mode: "steer" }`
- **THEN** synthetic events are emitted on the existing 200ms timer
- **AND** `createAgentSession` is never invoked
- **AND** boot does not fail even if `~/.pi/agent/auth.json` is absent
