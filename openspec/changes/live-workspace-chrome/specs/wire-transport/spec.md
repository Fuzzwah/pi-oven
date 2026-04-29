## MODIFIED Requirements

### Requirement: AgentStatus notifies client of session state changes

The server SHALL send `AgentStatus { workspace_id, status: "running" | "idle" }` when the pi session transitions state. The client SHALL use this to update the matching tab cell's `TabStatus` in `AppState.tabs` and to gate the Escape-to-abort shortcut. When the status is for the active workspace, the client SHALL also update `workspace_status` in `AppState`.

#### Scenario: AgentStatus round-trips through the wire codec

- **WHEN** the server serialises `AgentStatus { workspace_id: 1, status: "running" }` and the client deserialises it
- **THEN** the client has `workspace_id = 1` and `status = "running"`

#### Scenario: AgentStatus updates tab visual state

- **WHEN** the client receives `AgentStatus { workspace_id: 1, status: "running" }`
- **THEN** the tab cell for workspace 1 has `TabStatus::Active`
- **AND** the UI redraws on the next frame
