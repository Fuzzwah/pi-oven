## Requirements

### Requirement: Conversation pane renders a sequence of agent events

The conversation widget SHALL accept a list of `RenderedEvent` items in `AppState` and paint them as a scrollable conversation in the conversation pane area. Each event type SHALL have a distinct visual representation: text deltas are accumulated into assistant message bubbles; tool call events show the tool name and (collapsed) arguments; tool result events show the result (truncated at 10 lines with a fold indicator); unknown event types render as a grey raw JSON line.

#### Scenario: Text delta accumulates into an assistant bubble

- **WHEN** the conversation buffer contains a sequence of text-delta events forming the string "Hello, world!"
- **THEN** the conversation pane renders a single assistant message block containing "Hello, world!"
- **AND** the block is visually distinct from user messages (different foreground colour or prefix)

#### Scenario: Tool call renders with tool name

- **WHEN** the conversation buffer contains a tool-call event with `tool_name = "read_file"` and `args = { "path": "src/main.rs" }`
- **THEN** the pane renders a line like `▶ read_file` (tool name, visually muted)
- **AND** the arguments are not shown by default (collapsed)

#### Scenario: Tool result is truncated beyond 10 lines

- **WHEN** a tool result event contains output that spans 15 lines
- **THEN** the pane renders 10 lines followed by a `… 5 more lines` indicator
- **AND** no scrolling within the truncated block is required

#### Scenario: Unknown event type renders as raw fallback

- **WHEN** an `AgentEvent` carries an event type not recognised by the client
- **THEN** the pane renders a single grey line containing the raw JSON of the event
- **AND** rendering does not panic or produce layout corruption

### Requirement: Scroll-pinning during streaming (follow mode)

The conversation pane SHALL operate in follow mode by default: as new events are appended, the viewport scrolls to keep the latest content visible. Follow mode SHALL be suspended when the user scrolls up (viewport is not at the bottom). Scrolling back to the bottom SHALL re-engage follow mode.

#### Scenario: New events keep the viewport at the bottom in follow mode

- **WHEN** the conversation pane is in follow mode and new `AgentEvent` items are appended
- **THEN** the viewport scrolls so the most recent event is visible after each redraw

#### Scenario: Scrolling up suspends follow mode

- **WHEN** the user presses the up-arrow or Page-Up key while in follow mode
- **THEN** follow mode is suspended and the viewport does not auto-scroll on subsequent event appends

#### Scenario: Scrolling to bottom re-engages follow mode

- **WHEN** the user has scrolled up (follow mode suspended) and then scrolls back to the bottom
- **THEN** follow mode is re-engaged
- **AND** the next appended event causes the viewport to scroll

### Requirement: Tab characters in event text expanded to 8-column stops

Text content within `AgentEvent` payloads SHALL have tab characters (`\t`) expanded to spaces before being stored in the conversation buffer. Each `\t` SHALL be replaced by the number of spaces needed to reach the next 8-column boundary from the current column position.

#### Scenario: Tab at column 0 expands to 8 spaces

- **WHEN** an event text begins with `\t` followed by `code`
- **THEN** the rendered line begins with 8 spaces followed by `code`

#### Scenario: Tab at column 4 expands to 4 spaces

- **WHEN** an event text contains `abcd\tefgh`
- **THEN** the rendered output is `abcd    efgh` (4 spaces to reach column 8)

### Requirement: User message display

The conversation pane SHALL display user messages (from the `Send` action) inline in the conversation stream, above the assistant response they triggered. User messages SHALL be visually distinct from assistant messages (different prefix or colour).

#### Scenario: User message appears before assistant reply

- **WHEN** the user sends "What is 2+2?" and the assistant replies "4"
- **THEN** the conversation pane shows the user message "What is 2+2?" followed by the assistant message "4" in that order

#### Scenario: User message is styled differently from assistant message

- **WHEN** both a user message and an assistant message are in the conversation buffer
- **THEN** the user message uses a different visual style (prefix character or foreground colour) from the assistant message
