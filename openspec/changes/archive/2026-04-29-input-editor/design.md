## Context

The input bar currently stores text as `AppState.input: String` and handles keystroke mutations directly in `wgpu_main::App::handle_key`. There is no cursor position — the cursor is always rendered at the end — and there is no selection. The blinking cursor (just added) is purely cosmetic; it does not track where editing operations occur. Moving editing logic into a dedicated struct eliminates the ad-hoc string mutation in the event handler and provides a clear API boundary between "what the user typed" and "how it is displayed".

## Goals / Non-Goals

**Goals:**
- `InputEditor` lives in `pi-oven-ui` (shared between wgpu and crossterm backends).
- Tracks: text buffer (`String`), cursor position (byte index), optional selection (anchor byte index — the end that doesn't move when shift-extending).
- All operations work on Unicode scalar values (char boundaries), never splitting multibyte characters.
- `render_input` draws cursor at the correct column and a highlight span for the selection.
- `AppState` exposes `editor: InputEditor`; callers use `editor.text()` to read the string.

**Non-Goals:**
- Multi-line editing.
- Clipboard integration (paste/copy) — can be added later.
- IME / composition input.
- Undo/redo history.
- Vim or readline keymaps.

## Decisions

### D1 — Byte-index cursor, char-boundary invariant enforced at mutation sites

**Decision:** Store cursor as a byte offset into the UTF-8 string, not a char index.

**Rationale:** Rust's `String` is UTF-8 and all slice operations use byte indices. Storing a char index would require O(n) conversion on every operation. The invariant "cursor is always on a char boundary" is easy to maintain: every method that moves the cursor uses `char_indices()` or `floor_char_boundary` to find the next/previous boundary.

**Alternative considered:** `char` index — simpler mental model but slower and requires conversion at every render/edit site.

### D2 — Selection stored as anchor byte index (Option<usize>)

**Decision:** Selection is `Option<usize>` — the anchor (the end that stays fixed when shift-extending). The active end is always the cursor. The selected range is `min(anchor, cursor)..max(anchor, cursor)`.

**Rationale:** Matches how every terminal/text editor models selection internally. Starting a non-shift movement clears the anchor. Starting a shift-movement sets anchor to cursor position if it is not already set, then moves cursor.

**Alternative considered:** Storing `(start, end, direction)` — more explicit but adds a direction field that's always derivable from `anchor` vs `cursor`.

### D3 — Word boundary: whitespace-delimited

**Decision:** A "word" for Option+arrow is a run of non-whitespace characters. Moving right jumps to the start of the next whitespace gap after the current word; moving left jumps to the start of the current (or previous) word.

**Rationale:** Matches macOS Terminal and most Unix shells. Simple to implement with `char_indices()` scan; no need for Unicode word-break algorithm tables.

**Alternative considered:** Unicode word-break algorithm — more correct for mixed-script input but overkill for a command input bar.

### D4 — `InputEditor` in `pi-oven-ui`, not a separate crate

**Decision:** Implement `InputEditor` as a module inside `pi-oven-ui`.

**Rationale:** It is consumed exclusively by `pi-oven-ui`'s render functions and the binary's event handler. Extracting a crate would add build graph complexity for no gain at this stage.

### D5 — `render_input` takes `&InputEditor` instead of `(&str, cursor, selection)`

**Decision:** Pass the whole `InputEditor` to `render_input`.

**Rationale:** The render function needs text, cursor position, and selection simultaneously; passing them as separate arguments is noisy and will grow as features are added. A single reference is cleaner and matches how `AppState` stores it.

## Risks / Trade-offs

- **Byte-index bookkeeping:** Every mutation must re-clamp the cursor to a char boundary. If a method forgets, downstream renders or slices will panic. Mitigation: a private `clamp_cursor` helper called at the end of every mutating method; property-based tests covering random UTF-8 inputs.
- **Breaking `AppState.input`:** Any code reading `state.input` directly will fail to compile. Mitigation: the only consumers are `render_input` (updated as part of this change) and `main.rs` (updated as part of this change). The crossterm loop uses `app_state.input.push(c)` / `.pop()` — these become `editor` calls.
- **Selection rendering adds cells:** The selection highlight requires the text to be split into three spans (before-selection, selection, after-selection) with different styling. This increases the number of ratatui spans per render but is negligible in practice (the input bar is one row).

## Migration Plan

1. Add `InputEditor` struct and unit tests — no user-visible change.
2. Replace `AppState.input: String` with `AppState.editor: InputEditor` — compile-fails guide all call sites.
3. Update `render_input` to use `&InputEditor`.
4. Update `wgpu_main::handle_key` to call `InputEditor` methods.
5. Update `crossterm_main` key handling.
6. Delete old `state.input` references.

No data migration needed — the field is ephemeral (not persisted).

## Open Questions

- Should Delete (forward-delete) be exposed in the crossterm backend? crossterm sends `KeyCode::Delete` so yes, easy to add.
- Should Cmd+Backspace delete to line start or clear the entire field? macOS convention is "delete to line start" (not "clear all") — go with that.
