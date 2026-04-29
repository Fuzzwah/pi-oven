## Why

The client window currently paints a single `pi-oven` placeholder string. Before any real workspace, agent, or replay work can land, the four-pane chrome (sidebar, tab strip, conversation pane, input bar) needs to exist as empty shells so subsequent slices have somewhere to draw into. Doing the layout work now — under both `dev-wgpu` and `dev-crossterm` — also exercises the ratatui-backend split end-to-end, which is the load-bearing premise of the crate layout.

## What Changes

- Introduce four widget modules in `pi-oven-ui` (sidebar, tabs, conversation, input) as Backend-trait-agnostic ratatui widgets that draw labelled, empty frames.
- Add a top-level layout function in `pi-oven-ui` that arranges the four panes: fixed-width sidebar on the left; tab strip pinned to the top of the right column; input bar pinned to the bottom; conversation fills the remaining space.
- Replace the binary's single-`Paragraph` draw closure (in both `wgpu_main` and `crossterm_main`) with a call into the new layout function so both backends paint the same shells.
- Render placeholder labels inside each pane (e.g. `Projects`, `No workspaces`, `Conversation`, `>`) so the user can visually confirm the layout works at any window size.
- The shells are non-interactive in this slice: no focus model, no hotkeys beyond what already exists (`cmd+w` to quit), no real data.

## Capabilities

### New Capabilities

- `client-ui`: pane layout and widget tree for the pi-oven client. Owns sidebar, tab strip, conversation pane, and input bar widgets, plus the top-level layout that composes them. Backend-trait-agnostic — same widgets render under `dev-wgpu` and `dev-crossterm`.

### Modified Capabilities

(none — the existing `client-runtime` spec covers the window, render pipeline, and ratatui backend. This change consumes those primitives without changing their requirements.)

## Impact

- **New code**: modules under `crates/pi-oven-ui/src/` (sidebar, tabs, conversation, input, layout).
- **Modified code**: `crates/pi-oven/src/main.rs` — both `wgpu_main::App::redraw` and `crossterm_main::run` swap their hard-coded `Paragraph` for `pi_oven_ui::render(frame)` (or equivalent).
- **Dependencies**: `pi-oven-ui` already depends on `ratatui` per the scaffold-runtime layout; no new crates expected. `pi-oven` already depends on `pi-oven-ui` (currently unused).
- **Tests**: ratatui's `TestBackend` exercises the layout in `pi-oven-ui` unit tests at representative window sizes (small, medium, large). No new integration tests on the binary.
- **Manual verification**: `cargo run -p pi-oven` (wgpu) and `cargo run -p pi-oven --no-default-features --features dev-crossterm` both show the four labelled panes and resize cleanly.
