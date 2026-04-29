## Why

The input bar currently owns its text as a plain `String` in `AppState` with editing handled ad-hoc in `main.rs`. There is no cursor position, no selection, and no word-boundary movement — making the bar feel unfinished compared to any real terminal. Introducing a self-contained `InputEditor` struct gives the bar proper single-line editing semantics and decouples editor logic from both the rendering layer and the platform event handler.

## What Changes

- **New struct `InputEditor`** in `pi-oven-ui` that owns the text buffer, cursor byte position, and optional selection anchor — with methods covering every editing operation.
- `AppState.input: String` replaced by `AppState.editor: InputEditor`; rendering reads from `InputEditor` instead of a bare string.
- `render_input` updated to draw the cursor at the correct position and render a selection highlight when a selection is active.
- `main.rs` key handler updated to call `InputEditor` methods for all editing actions (arrow keys, Option/Cmd modifiers, Shift-selection, Delete, Backspace variants).
- Crossterm dev backend receives the same `InputEditor` methods for the subset of keys crossterm can intercept.

## Capabilities

### New Capabilities

- `input-editor`: Single-line text editor state machine — text buffer, cursor position, selection range, and all editing operations (char/word/line movement + deletion, selection).

### Modified Capabilities

- (none — no existing spec-level requirements change)

## Impact

- `crates/pi-oven-ui/src/lib.rs` — `AppState` struct shape changes (`input: String` → `editor: InputEditor`); **BREAKING** for any code that reads `state.input` directly.
- `crates/pi-oven-ui/src/input.rs` — `render_input` signature and rendering logic.
- `crates/pi-oven/src/main.rs` — key handler in `wgpu_main` and `crossterm_main` loops.
- No new crate dependencies required.
