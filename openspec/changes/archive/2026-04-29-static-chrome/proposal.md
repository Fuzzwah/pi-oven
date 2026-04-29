## Why

The right pane's empty shells (`tabs`, `conversation`, `input`) currently render as bare bordered boxes with placeholder labels. The target client UX (as seen in the reference screenshot) frames those panes with four additional pieces of static chrome: a **conversation header** above the message area, a **bottom status bar** below the input, a **hotkey legend** at the very bottom, and **tab badges with status indicators** on each tab. None of these need real workspace data — they're pure presentation. Landing them now closes most of the visual gap to the target without waiting on wire-transport plumbing, and gives later slices (message rendering, focus model, real workspace state) somewhere to draw into.

## What Changes

- Add a **conversation header strip** between the tab strip and the conversation body: a title row (e.g. `Apply clipboard support changes`) and a small stats sub-row (e.g. `51s · ↓7 ↑2.3k`). Both populated from `AppState` placeholders for now.
- Add a **status bar** below the input bar showing model name, ctx%, branch name, and PR badge — single line, all from `AppState` placeholders.
- Add a **hotkey legend strip** at the very bottom of the window: a single non-bordered row of key/action pairs that mirrors the legend in the reference screenshot.
- Modify the **tab strip** to render a list of mocked workspace tabs (one per `AppState.tabs[]` placeholder entry), each showing: status dot, hotkey index `[N]`, project name, worktree name, and an optional badge (PR number or unread count).
- Extend `AppState` with placeholder fields backing the new chrome (header title, stats, status-bar values, tab list). No event plumbing — fields are populated with hard-coded demo values in `AppState::default()` so the visual lands without any wire dependency.
- Adjust the top-level layout in `pi-oven-ui::render` to slot in the two new strips (header above conversation, status + legend below input). Sidebar and existing pane positions are unchanged.

## Capabilities

### New Capabilities

(none — all changes belong to the existing `client-ui` capability)

### Modified Capabilities

- `client-ui`: layout grows two new strips (conversation header, bottom status + legend); tab strip widget changes from a single placeholder line to a list of tab cells with badges; new requirements for placeholder-state fields on `AppState`.

## Impact

- **Modified code**:
  - `crates/pi-oven-ui/src/lib.rs` — `AppState` gains placeholder fields; `render` adds the two new strips to the layout.
  - `crates/pi-oven-ui/src/tabs.rs` — replace the single `(no workspaces)` line with a tab-cell renderer driven by `AppState.tabs`.
  - `crates/pi-oven-ui/src/conversation.rs` — header strip becomes its own widget (or a separate module `header.rs`); the conversation body keeps its current `(empty)` placeholder.
- **New code**:
  - `crates/pi-oven-ui/src/header.rs` — conversation header (title + stats sub-row).
  - `crates/pi-oven-ui/src/status_bar.rs` — bottom status bar (model · ctx · branch · PR).
  - `crates/pi-oven-ui/src/legend.rs` — hotkey legend row.
- **Dependencies**: none. All four pieces are pure ratatui widgets.
- **Tests**: extend the existing ratatui `TestBackend` snapshot tests in `pi-oven-ui` to cover the new strips at representative window sizes (small, medium, wide).
- **Manual verification**: `cargo run -p pi-oven` (wgpu) and `cargo run -p pi-oven --no-default-features --features dev-crossterm` both show the populated chrome at any reasonable window size.
- **Out of scope** (deliberately deferred):
  - Real workspace/tab data from the server. Placeholders only.
  - Focus model and hotkey routing for tab switching, model selection, etc. The legend is descriptive, not functional.
  - Conversation message bubbles and rich-text rendering. Body stays `(empty)`.
  - Sidebar redesign (project list with status indicators). Sidebar keeps its current placeholder.
