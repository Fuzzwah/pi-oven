## ADDED Requirements

### Requirement: Top-level pane layout

The client UI SHALL compose four panes — sidebar, tab strip, conversation, input bar — into a single layout that fills the available render area. The layout SHALL be exposed from `pi-oven-ui` as a single render entrypoint that the binary calls inside ratatui's draw closure.

#### Scenario: Single render entrypoint exists

- **WHEN** the binary calls `pi_oven_ui::render(frame)` inside `terminal.draw(|f| ...)`
- **THEN** the four panes are drawn into `frame`'s buffer
- **AND** no further per-pane calls are required from the binary

#### Scenario: Sidebar is fixed-width on the left

- **WHEN** the layout is rendered into any area at least 40 columns wide
- **THEN** the sidebar pane occupies the leftmost 28 columns
- **AND** the remaining columns form the right column containing the tab strip, conversation pane, and input bar

#### Scenario: Tab strip pinned to top of right column

- **WHEN** the layout is rendered
- **THEN** the tab strip occupies the top 3 rows of the right column

#### Scenario: Input bar pinned to bottom of right column

- **WHEN** the layout is rendered
- **THEN** the input bar occupies the bottom 3 rows of the right column

#### Scenario: Conversation pane fills the remainder

- **WHEN** the layout is rendered
- **THEN** the conversation pane occupies all rows in the right column not consumed by the tab strip or input bar

#### Scenario: Layout adapts to window resize

- **WHEN** the render area changes size between draw calls
- **THEN** the four panes are recomputed against the new area on the next draw without panic
- **AND** sidebar width and input/tab bar heights remain at their fixed values; the conversation pane absorbs the change

### Requirement: Backend-trait-agnostic widgets

All widgets in `pi-oven-ui` SHALL be written against `ratatui::backend::Backend` and MUST NOT reference any concrete backend type, so the same code paints under both `dev-wgpu` and `dev-crossterm`.

#### Scenario: Widgets render under dev-wgpu

- **WHEN** the binary built with default features (`dev-wgpu`) calls `pi_oven_ui::render(frame)`
- **THEN** the four panes are visible in the native window

#### Scenario: Widgets render under dev-crossterm

- **WHEN** the binary built with `--no-default-features --features dev-crossterm` calls `pi_oven_ui::render(frame)`
- **THEN** the four panes are visible in the host terminal

#### Scenario: pi-oven-ui has no concrete-backend dependency

- **WHEN** `cargo metadata` is consulted for `pi-oven-ui`
- **THEN** its dependency tree contains neither `pi-oven-render` nor `crossterm`

### Requirement: Pane shells with placeholder content

Each of the four panes SHALL render a bordered shell with a placeholder body so the layout is legible before any real state is wired in.

#### Scenario: Sidebar shows projects placeholder

- **WHEN** the layout is rendered
- **THEN** the sidebar pane is bordered, has the title `Projects`, and contains the body `(no projects)`

#### Scenario: Tab strip shows no-workspaces placeholder

- **WHEN** the layout is rendered with no workspaces present
- **THEN** the tab strip pane is bordered and contains the body `(no workspaces)`

#### Scenario: Conversation pane shows empty placeholder

- **WHEN** the layout is rendered
- **THEN** the conversation pane is bordered, has the title `Conversation`, and contains the body `(empty)`

#### Scenario: Input bar shows prompt placeholder

- **WHEN** the layout is rendered
- **THEN** the input bar pane is bordered and contains the body `>`

### Requirement: Layout is non-interactive in this slice

The layout SHALL be visually present without a focus model, hotkey routing, or scrollable content. Existing client hotkeys (e.g. `Cmd+W` to quit) MUST continue to work unchanged.

#### Scenario: No pane is highlighted as focused

- **WHEN** the client window has keyboard focus
- **THEN** no pane renders a focus indicator (highlighted border, accent colour, etc.)

#### Scenario: Cmd+W still quits

- **WHEN** the user presses `Cmd+W` while the client window has focus
- **THEN** the event loop exits with status zero, exactly as before this change
