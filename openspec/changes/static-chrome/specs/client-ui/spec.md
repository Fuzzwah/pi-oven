## MODIFIED Requirements

### Requirement: Top-level pane layout

The client UI SHALL compose six regions — sidebar, tab strip, conversation header, conversation body, input bar, and bottom strip — into a single layout that fills the available render area. The layout SHALL be exposed from `pi-oven-ui` as a single render entrypoint that the binary calls inside ratatui's draw closure. The sidebar occupies the leftmost 28 columns; the remaining columns form the right column whose vertical strips are, top-to-bottom: tab strip (3 rows), conversation header (3 rows), conversation body (remainder), input bar (3 rows), bottom strip (3 rows containing status bar and hotkey legend).

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

#### Scenario: Conversation header sits below the tab strip

- **WHEN** the layout is rendered
- **THEN** the conversation header occupies rows 4–6 of the right column (zero-indexed: rows 3..6)

#### Scenario: Bottom strip pinned to the bottom of the right column

- **WHEN** the layout is rendered
- **THEN** the bottom strip occupies the bottom 3 rows of the right column

#### Scenario: Input bar sits above the bottom strip

- **WHEN** the layout is rendered
- **THEN** the input bar occupies the 3 rows immediately above the bottom strip

#### Scenario: Conversation body fills the remainder

- **WHEN** the layout is rendered
- **THEN** the conversation body occupies all rows in the right column not consumed by the tab strip, conversation header, input bar, or bottom strip

#### Scenario: Layout adapts to window resize

- **WHEN** the render area changes size between draw calls
- **THEN** the six regions are recomputed against the new area on the next draw without panic
- **AND** sidebar width and the four fixed-height strips (tabs, header, input, bottom) remain at their fixed values; the conversation body absorbs the change

#### Scenario: Layout degrades gracefully on small windows

- **WHEN** the layout is rendered into an area shorter than 13 rows (the total fixed-strip height plus a single conversation row)
- **THEN** strips are still drawn in order from the top until vertical space is exhausted
- **AND** no panic occurs; remaining strips are clipped or omitted by ratatui's normal layout behaviour

### Requirement: Pane shells with placeholder content

Each region SHALL render a placeholder body keyed off `AppState` so the layout is legible before any wire data is connected. The four pre-existing panes (sidebar, tab strip, conversation body, input bar) keep bordered shells; the conversation header and bottom strip MAY use lighter framing as defined by their own requirements.

#### Scenario: Sidebar shows projects placeholder

- **WHEN** the layout is rendered
- **THEN** the sidebar pane is bordered, has the title `Projects`, and contains the body `(no projects)`

#### Scenario: Tab strip shows mocked workspace tabs

- **WHEN** the layout is rendered with one or more entries in `AppState.tabs`
- **THEN** the tab strip pane renders a tab cell for each entry as defined by the **Tab strip cells** requirement
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

`AppState` SHALL carry typed placeholder fields backing the conversation header, status bar, tab strip cells, and hotkey legend. `AppState::default()` SHALL populate these fields with hard-coded demo values that match the visual silhouette in the project's reference UX, so the layout renders meaningful content without any wire transport.

#### Scenario: AppState exposes header placeholders

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** the resulting state has a `header` field holding a non-empty title string and numeric values for elapsed time and token counts

#### Scenario: AppState exposes status-bar placeholders

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** the resulting state has a `status` field holding a non-empty model name, a `ctx_pct` value in `0..=100`, a non-empty branch string, and either `Some(pr)` or `None`

#### Scenario: AppState exposes tab cells

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** the resulting state has a `tabs` field holding two or more tab cells with distinct `idx` values and at least one of each `TabStatus` variant (`Active`, `Idle`)

#### Scenario: AppState exposes legend entries

- **WHEN** code in `pi-oven-ui` constructs `AppState::default()`
- **THEN** the resulting state has a `legend` field holding a non-empty ordered list of `(keys, action)` pairs

### Requirement: Conversation header strip

The conversation header strip SHALL render a 3-row pane between the tab strip and the conversation body. Row 1 is reserved for top spacing or a separator. Row 2 SHALL display the header title from `AppState.header.title` left-aligned. Row 3 SHALL display a status sub-row composed of the elapsed time and token counts from `AppState.header`, formatted as a single line of segments separated by `·`.

#### Scenario: Header renders the title

- **WHEN** the layout is rendered with `AppState.header.title = "Apply clipboard support changes"`
- **THEN** the conversation header strip displays that exact text on its title row, left-aligned within the strip

#### Scenario: Header renders elapsed time and token counts

- **WHEN** the layout is rendered with `AppState.header` set to `{ elapsed_secs: 51, tokens_in: 7, tokens_out: 2300, .. }`
- **THEN** the conversation header strip displays a status sub-row containing `51s`, `↓7`, and `↑2.3k` (or `↑2300`), each as a distinct segment separated by `·`

#### Scenario: Header truncates a long title

- **WHEN** the title is wider than the available column count
- **THEN** the rendered title is truncated with a trailing `…` so it fits within the strip's width without wrapping

### Requirement: Tab strip cells

When `AppState.tabs` is non-empty, the tab strip SHALL render one cell per entry, packed left-to-right with a separator between cells. Each cell SHALL include, in order: a status dot keyed to the cell's `TabStatus` (`▶` for `Active`, `•` for `Idle`, `!` for `Attention`); the bracketed index `[N]` from the cell's `idx`; the project name; the worktree name in parentheses; and an optional badge.

#### Scenario: Cell shows status dot, index, project, and worktree

- **WHEN** the layout is rendered with one tab cell `{ idx: 2, project: "pi-oven", worktree: "spry-mare", status: Active, badge: None }`
- **THEN** the tab strip displays a cell containing the substring `▶ [2] pi-oven (spry-mare)` in that order

#### Scenario: Idle cells use the idle dot

- **WHEN** the layout is rendered with one tab cell whose `status` is `Idle`
- **THEN** that cell's status dot is `•`, not `▶`

#### Scenario: Cell renders a PR badge

- **WHEN** the layout is rendered with one tab cell whose `badge` is `TabBadge::Pr(9)`
- **THEN** that cell's rendered line contains the substring `#9`

#### Scenario: Cell renders an unread badge

- **WHEN** the layout is rendered with one tab cell whose `badge` is `TabBadge::Unread { up: 3, down: 2 }`
- **THEN** that cell's rendered line contains the substring `↑3` and the substring `↓2`

#### Scenario: Tab strip truncates rightmost cells when overflowing

- **WHEN** the total rendered width of all tab cells exceeds the available column count
- **THEN** cells closer to the left edge are preserved
- **AND** the rightmost overflowing cells are replaced by a trailing ellipsis indicator (`…`) without panicking or wrapping

### Requirement: Bottom status bar

The bottom strip's status bar SHALL render a single row of `·`-separated segments showing, in order: the model name from `AppState.status.model`; the literal `ctx:NN%` formed from `AppState.status.ctx_pct`; the PR badge `PR #NN` when `AppState.status.pr` is `Some`; and the branch name from `AppState.status.branch`.

#### Scenario: Status bar shows model and ctx%

- **WHEN** the layout is rendered with `AppState.status.model = "Sonnet 4.6"` and `ctx_pct = 48`
- **THEN** the status bar contains the substrings `Sonnet 4.6` and `ctx:48%`, in that left-to-right order, separated by content that includes `·`

#### Scenario: Status bar shows PR badge when present

- **WHEN** the layout is rendered with `AppState.status.pr = Some(9)`
- **THEN** the status bar contains the substring `PR #9`

#### Scenario: Status bar omits PR badge when absent

- **WHEN** the layout is rendered with `AppState.status.pr = None`
- **THEN** the status bar contains no `PR #` substring

#### Scenario: Status bar shows branch name

- **WHEN** the layout is rendered with `AppState.status.branch = "fuz/apply-clipboard-support"`
- **THEN** the status bar contains the substring `fuz/apply-clipboard-support`

### Requirement: Hotkey legend

The bottom strip's hotkey legend SHALL render a single row beneath the status bar showing each entry from `AppState.legend` as `<keys> <action>` pairs separated by whitespace. The legend is descriptive only; this slice does not route any of the listed hotkeys.

#### Scenario: Legend renders all entries

- **WHEN** the layout is rendered with `AppState.legend` containing the pairs `("M-tab/M-S-tab", "next/prev tab")` and `("C-q", "quit")`
- **THEN** the legend row contains both pairs in left-to-right order, with the keys and action of each pair adjacent

#### Scenario: Legend truncates when overflowing

- **WHEN** the rendered legend would exceed the available column count
- **THEN** the rightmost entries are truncated with a trailing `…` so the row fits without wrapping

#### Scenario: Legend hotkeys are not interactive in this slice

- **WHEN** the user presses any key listed in the legend (other than `Cmd+W`, which retains its existing quit behaviour)
- **THEN** the layout state is unchanged
- **AND** no error is produced

### Requirement: New chrome is non-interactive in this slice

The conversation header, tab strip cells, status bar, and hotkey legend SHALL be visually present without focus indicators, hotkey routing, or selection state. Existing client hotkeys (`Cmd+W` to quit, `Cmd+C`/`Cmd+V`/`Cmd+X` clipboard handling on the input bar) MUST continue to work unchanged.

#### Scenario: No chrome region renders a focus indicator

- **WHEN** the client window has keyboard focus
- **THEN** none of the new chrome regions render a focused-pane indicator (highlighted border, accent colour, etc.)

#### Scenario: Cmd+W still quits

- **WHEN** the user presses `Cmd+W` while the client window has focus
- **THEN** the event loop exits with status zero, exactly as before this change

#### Scenario: Existing clipboard shortcuts unchanged

- **WHEN** the user presses `Cmd+C`, `Cmd+X`, or `Cmd+V` while the input bar contains text or a selection
- **THEN** copy/cut/paste behaves exactly as defined in the `clipboard` capability
