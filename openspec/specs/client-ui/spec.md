## MODIFIED Requirements

### Requirement: Top-level pane layout

The client UI SHALL compose six regions — sidebar, tab strip, conversation header, conversation body, input bar, and bottom strip — into a single layout that fills the available render area. The layout SHALL be exposed from `pi-oven-ui` as a single render entrypoint that the binary calls inside ratatui's draw closure. The sidebar occupies the leftmost 28 columns; the remaining columns form the right column whose vertical strips are, top-to-bottom: tab strip (3 rows), conversation header (1 row), conversation body (remainder), input bar (3 rows), bottom strip (2 rows containing status bar and hotkey legend, with no leading spacer).

#### Scenario: Single render entrypoint exists

- **WHEN** the binary calls `pi_oven_ui::render(frame, &state)` inside `terminal.draw(|f| ...)`
- **THEN** all six regions are drawn into `frame`'s buffer
- **AND** no further per-pane calls are required from the binary

#### Scenario: Sidebar is fixed-width on the left

- **WHEN** the layout is rendered into any area at least 40 columns wide
- **THEN** the sidebar pane occupies the leftmost 28 columns
- **AND** the remaining columns form the right column containing the five vertical strips

#### Scenario: Tab strip occupies the top of the right column

- **WHEN** the layout is rendered
- **THEN** the tab strip occupies rows 1–3 of the right column (zero-indexed: rows 0..3)

#### Scenario: Conversation header sits directly below the tab strip

- **WHEN** the layout is rendered
- **THEN** the conversation header occupies the single row immediately below the tab strip's bottom border (zero-indexed: row 3)
- **AND** there is no blank spacer row between the tab strip and the header

#### Scenario: Bottom strip pinned to the bottom of the right column

- **WHEN** the layout is rendered
- **THEN** the bottom strip occupies the bottom 2 rows of the right column

#### Scenario: Input bar sits directly above the bottom strip

- **WHEN** the layout is rendered
- **THEN** the input bar occupies the 3 rows immediately above the bottom strip
- **AND** there is no blank spacer row between the input bar and the bottom strip

#### Scenario: Conversation body fills the remainder

- **WHEN** the layout is rendered
- **THEN** the conversation body occupies all rows in the right column not consumed by the tab strip, conversation header, input bar, or bottom strip

#### Scenario: Layout adapts to window resize

- **WHEN** the render area changes size between draw calls
- **THEN** the six regions are recomputed against the new area on the next draw without panic
- **AND** sidebar width and the four fixed-height strips (tabs 3, header 1, input 3, bottom 2) remain at their fixed values; the conversation body absorbs the change

#### Scenario: Layout degrades gracefully on small windows

- **WHEN** the layout is rendered into an area shorter than 10 rows (the total fixed-strip height plus a single conversation row)
- **THEN** strips are still drawn in order from the top until vertical space is exhausted
- **AND** no panic occurs; remaining strips are clipped or omitted by ratatui's normal layout behaviour

### Requirement: Pane shells with placeholder content

Each region SHALL render a placeholder body keyed off `AppState` so the layout is legible before any wire data is connected. The four pre-existing panes (sidebar, tab strip, conversation body, input bar) keep bordered shells; the conversation header and bottom strip render unframed text rows.

#### Scenario: Sidebar shows projects placeholder

- **WHEN** the layout is rendered
- **THEN** the sidebar pane is bordered, has the title `Projects`, and contains the body `(no projects)`

#### Scenario: Tab strip shows mocked workspace tabs

- **WHEN** the layout is rendered with one or more entries in `AppState.tabs`
- **THEN** the tab strip pane renders cells for each entry as defined by the **Tab strip cells** requirement
- **AND** the empty-state placeholder is not shown

#### Scenario: Tab strip shows no-workspaces placeholder when empty

- **WHEN** the layout is rendered with `AppState.tabs` empty
- **THEN** the tab strip pane is bordered and contains the body `(no workspaces)`

#### Scenario: Conversation body shows empty placeholder

- **WHEN** the layout is rendered
- **THEN** the conversation body pane is bordered, has the title `Conversation`, and contains the body `(empty)`

#### Scenario: Input bar shows prompt placeholder

- **WHEN** the layout is rendered with an empty editor
- **THEN** the input bar pane is bordered and contains the prompt prefix `> ` followed by the editor's cursor

## ADDED Requirements

### Requirement: Application state carries chrome placeholders

`AppState` SHALL carry typed placeholder fields backing the conversation header, status bar, tab strip cells, and hotkey legend. `AppState::default()` SHALL populate these fields with hard-coded generic placeholder values (e.g. `[project N]`, `[Model]`, `[branch name]`) so the layout renders meaningful content without any wire transport, while making it visually obvious that the values are not real session data.

#### Scenario: AppState exposes a header title

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** the resulting state has a `header` field holding a non-empty title string

#### Scenario: AppState exposes status-bar placeholders

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** the resulting state has a `status` field holding non-empty `model`, `ctx`, and `branch` strings, plus an `Option`-typed `pr` that is `Some` for the demo

#### Scenario: AppState exposes tab cells

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** the resulting state has a `tabs` field holding two or more tab cells with distinct `idx` values and at least one of each `TabStatus` variant (`Active`, `Idle`)

#### Scenario: AppState exposes legend entries

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** the resulting state has a `legend` field holding a non-empty ordered list of `(keys, action)` pairs that correspond to hotkeys actually wired in the binary

### Requirement: Conversation header strip

The conversation header strip SHALL render a single row directly below the tab strip displaying the title from `AppState.header.title`, centered and bold. There SHALL be no separate stats sub-row or leading spacer in this slice.

#### Scenario: Header renders a centered, bold title

- **WHEN** the layout is rendered with `AppState.header.title = "[project 1] Longer Explanation of Feature xyz"`
- **THEN** the conversation header strip displays that exact text on its single row, centered within the strip's width and styled bold

#### Scenario: Header truncates a long title

- **WHEN** the title is wider than the available column count
- **THEN** the rendered title is truncated with a trailing `…` so it fits within the strip's width without wrapping

### Requirement: Tab strip cells

When `AppState.tabs` is non-empty, the tab strip SHALL render one cell per entry, packed left-to-right with separators that vary by the next cell's status. Each cell SHALL be of the form `[<project>] (<trigger>)`, where `<project>` and `<trigger>` come from the cell's fields and the project segment is rendered bold for `Active` (and `Attention`) cells. The separator before a cell whose status is `Active` (or `Attention`) SHALL be ` > `; the separator before any other cell SHALL be ` - `. The first cell has no leading separator.

#### Scenario: Cell shows project and trigger

- **WHEN** the layout is rendered with one tab cell `{ idx: 1, project: "project 1", trigger: "issue-123", status: Idle }`
- **THEN** the tab strip displays a cell containing the substring `[project 1] (issue-123)` in that order

#### Scenario: Active separator points at the active cell

- **WHEN** the layout is rendered with two tab cells where the second has `status: Active`
- **THEN** the tab strip rendered line contains the substring ` > ` between the two cells
- **AND** does not contain ` - ` between those cells

#### Scenario: Idle separator joins consecutive idle cells

- **WHEN** the layout is rendered with three or more idle cells in sequence
- **THEN** the tab strip rendered line contains ` - ` between each pair of consecutive idle cells

#### Scenario: Tab strip truncates rightmost cells when overflowing

- **WHEN** the total rendered width of all tab cells and separators exceeds the available column count
- **THEN** cells closer to the left edge are preserved
- **AND** the rightmost overflowing cells are replaced by a trailing ellipsis indicator (`…`) without panicking or wrapping

### Requirement: Bottom status bar

The bottom strip's status bar SHALL render a single bold row of `-`-separated segments showing, in order: the model from `AppState.status.model`; the context value from `AppState.status.ctx`; the PR badge `PR# <pr>` when `AppState.status.pr` is `Some`; and the branch from `AppState.status.branch`. The entire row SHALL be styled bold.

#### Scenario: Status bar shows model and context

- **WHEN** the layout is rendered with `AppState.status.model = "[Model]"` and `AppState.status.ctx = "[context %]"`
- **THEN** the status bar row contains the substrings `[Model]` and `[context %]`, in that left-to-right order, separated by ` - `

#### Scenario: Status bar shows PR badge when present

- **WHEN** the layout is rendered with `AppState.status.pr = Some("[123]")`
- **THEN** the status bar contains the substring `PR# [123]`

#### Scenario: Status bar omits PR badge when absent

- **WHEN** the layout is rendered with `AppState.status.pr = None`
- **THEN** the status bar contains no `PR#` substring

#### Scenario: Status bar shows branch

- **WHEN** the layout is rendered with `AppState.status.branch = "[branch name]"`
- **THEN** the status bar contains the substring `[branch name]`

### Requirement: Hotkey legend

The bottom strip's hotkey legend SHALL render a single row beneath the status bar showing each entry from `AppState.legend` as `<keys> <action>` pairs separated by whitespace. The legend SHALL list only hotkeys that are actually wired in the binary so it doubles as a check on what works today; this slice does not route any new hotkeys via the legend itself.

#### Scenario: Legend renders entries from real hotkeys

- **WHEN** the layout is rendered with `AppState.legend` populated by `AppState::default()`
- **THEN** the legend row contains the substring `Cmd+W` paired with `quit`
- **AND** the legend row contains a substring identifying the clipboard hotkeys (e.g. `Cmd+C`, `Cmd+V`)

#### Scenario: Legend truncates when overflowing

- **WHEN** the rendered legend would exceed the available column count
- **THEN** the rightmost entries are truncated with a trailing `…` so the row fits without wrapping

#### Scenario: Legend hotkeys retain their existing wiring

- **WHEN** the user presses any hotkey listed in the legend
- **THEN** the existing handler in the binary continues to execute as before this change
- **AND** the legend itself does not introduce new key handlers

### Requirement: New chrome is non-interactive in this slice

The conversation header, tab strip cells, status bar, and hotkey legend SHALL be visually present without focus indicators, hotkey routing, or selection state. Existing client hotkeys (`Cmd+W` to quit, `Cmd+=`/`Cmd+-` to adjust font size, `Cmd+C`/`Cmd+V`/`Cmd+X` clipboard handling on the input bar) MUST continue to work unchanged.

#### Scenario: No chrome region renders a focus indicator

- **WHEN** the client window has keyboard focus
- **THEN** none of the new chrome regions render a focused-pane indicator (highlighted border, accent colour, etc.)

#### Scenario: Cmd+W still quits

- **WHEN** the user presses `Cmd+W` while the client window has focus
- **THEN** the event loop exits with status zero, exactly as before this change

#### Scenario: Existing clipboard shortcuts unchanged

- **WHEN** the user presses `Cmd+C`, `Cmd+X`, or `Cmd+V` while the input bar contains text or a selection
- **THEN** copy/cut/paste behaves exactly as defined in the `clipboard` capability

### Requirement: Input bar clipboard shortcuts (dev-wgpu)

The input bar in the `dev-wgpu` backend SHALL handle `Cmd+C`, `Cmd+X`, and `Cmd+V` to trigger clipboard copy, cut, and paste respectively. These shortcuts SHALL be dispatched through the existing `CmdLetter` translation path in `keys.rs`.

#### Scenario: Cmd+C is dispatched as copy in wgpu backend

- **WHEN** the user presses `Cmd+C` while the wgpu window has focus
- **THEN** `handle_key` routes the event to the clipboard copy operation
- **AND** the cursor blink is reset and the frame is redrawn

#### Scenario: Cmd+X is dispatched as cut in wgpu backend

- **WHEN** the user presses `Cmd+X` while the wgpu window has focus
- **THEN** `handle_key` routes the event to the clipboard cut operation
- **AND** the cursor blink is reset and the frame is redrawn

#### Scenario: Cmd+V is dispatched as paste in wgpu backend

- **WHEN** the user presses `Cmd+V` while the wgpu window has focus
- **THEN** `handle_key` routes the event to the clipboard paste operation
- **AND** the cursor blink is reset and the frame is redrawn

### Requirement: Input bar clipboard shortcuts (dev-crossterm)

The input bar in the `dev-crossterm` backend SHALL handle `Ctrl+C`, `Ctrl+X`, and `Ctrl+V` as the terminal equivalents for clipboard copy, cut, and paste.

#### Scenario: Ctrl+C copies in crossterm backend

- **WHEN** the user presses `Ctrl+C` in the crossterm backend
- **THEN** the selected text (if any) is written to the system clipboard
- **AND** the application does not exit

#### Scenario: Ctrl+X cuts in crossterm backend

- **WHEN** the user presses `Ctrl+X` in the crossterm backend
- **THEN** the selected text (if any) is cut to the system clipboard

#### Scenario: Ctrl+V pastes in crossterm backend

- **WHEN** the user presses `Ctrl+V` in the crossterm backend
- **THEN** clipboard text is inserted at the cursor position
