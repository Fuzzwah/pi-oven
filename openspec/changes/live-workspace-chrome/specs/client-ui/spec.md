## MODIFIED Requirements

### Requirement: Application state carries chrome placeholders

`AppState` SHALL carry typed fields backing the conversation header, status bar, tab strip cells, hotkey legend, and active workspace tracking. `AppState::default()` SHALL produce a blank connecting state (no mock tabs, `header.title = "Connecting…"`, status fields `"–"`) so that placeholder values are never confused with real server data. After `Welcome` is received the fields SHALL be populated from server-supplied workspace data.

`AppState` SHALL include an `active_workspace_id: u64` field (default `0`, sentinel meaning no workspace yet) identifying which workspace the header and status bar currently reflect.

#### Scenario: AppState default has no tabs

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** the resulting state has `tabs` equal to an empty `Vec`

#### Scenario: AppState default shows connecting header

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** `header.title` is `"Connecting…"`

#### Scenario: AppState default shows dash placeholders in status bar

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** `status.model`, `status.ctx`, and `status.branch` are each `"–"`
- **AND** `status.pr` is `None`

#### Scenario: AppState exposes active_workspace_id

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** the resulting state has an `active_workspace_id` field equal to `0`

#### Scenario: AppState exposes legend entries

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** the resulting state has a `legend` field holding a non-empty ordered list of `(keys, action)` pairs that correspond to hotkeys actually wired in the binary

## ADDED Requirements

### Requirement: Tab strip populated from Welcome workspaces

On receiving `Welcome`, the client SHALL replace `AppState.tabs` with one `TabCell` per entry in `workspaces`, in order. The `active_workspace_id` SHALL be set to the first workspace's `workspace_id` (or remain `0` if `workspaces` is empty). Each cell's initial `TabStatus` SHALL be derived from the snapshot's `status` field (`"running"` → `Active`, anything else → `Idle`).

#### Scenario: Single workspace produces one tab cell

- **WHEN** the client receives `Welcome { workspaces: [{ workspace_id: 1, status: "idle" }] }`
- **THEN** `AppState.tabs` has exactly one entry with `idx = 1`
- **AND** `active_workspace_id` is `1`

#### Scenario: Multiple workspaces produce multiple tab cells

- **WHEN** the client receives `Welcome { workspaces: [{ workspace_id: 1, status: "idle" }, { workspace_id: 2, status: "running" }] }`
- **THEN** `AppState.tabs` has exactly two entries
- **AND** the cell for workspace 2 has `TabStatus::Active`

#### Scenario: Empty workspaces clears tab strip

- **WHEN** the client receives `Welcome { workspaces: [] }`
- **THEN** `AppState.tabs` is empty
- **AND** `active_workspace_id` is `0`

#### Scenario: Welcome replaces any existing tabs

- **WHEN** the client has existing tabs from a prior connection and receives a new `Welcome`
- **THEN** `AppState.tabs` is rebuilt entirely from the new `workspaces` list

### Requirement: AgentStatus updates tab status in real time

On receiving `AgentStatus { workspace_id, status }`, the client SHALL find the `TabCell` in `AppState.tabs` with matching `idx` and update its `TabStatus` to `Active` when `status == "running"` or `Idle` otherwise.

#### Scenario: Running status marks tab Active

- **WHEN** the client receives `AgentStatus { workspace_id: 1, status: "running" }`
- **AND** `AppState.tabs` contains a cell with `idx = 1`
- **THEN** that cell's `TabStatus` is `Active`

#### Scenario: Idle status marks tab Idle

- **WHEN** the client receives `AgentStatus { workspace_id: 1, status: "idle" }`
- **AND** the cell for workspace 1 was previously `Active`
- **THEN** that cell's `TabStatus` is `Idle`

#### Scenario: AgentStatus for unknown workspace is ignored

- **WHEN** the client receives `AgentStatus` for a `workspace_id` not in `AppState.tabs`
- **THEN** `AppState.tabs` is unchanged and no panic occurs
