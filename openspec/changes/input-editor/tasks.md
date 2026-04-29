## 1. InputEditor struct

- [x] 1.1 Create `crates/pi-oven-ui/src/editor.rs` with `InputEditor` struct (fields: `buf: String`, `cursor: usize`, `anchor: Option<usize>`) and `pub use` from `lib.rs`
- [x] 1.2 Implement `Default`, `text()`, `cursor_byte_pos()`, `selection()` (returns ordered `Option<(usize, usize)>`)
- [x] 1.3 Implement `push_str(s: &str)` — insert at cursor, advance cursor
- [x] 1.4 Implement `move_right(extend: bool)` and `move_left(extend: bool)` with selection-collapse logic
- [x] 1.5 Implement `move_word_right(extend: bool)` and `move_word_left(extend: bool)` using whitespace word boundaries
- [x] 1.6 Implement `move_to_start(extend: bool)` and `move_to_end(extend: bool)`
- [x] 1.7 Implement `delete_before()` — backspace; deletes selection if active
- [x] 1.8 Implement `delete_after()` — forward delete
- [x] 1.9 Implement `delete_word_before()` — Option+Backspace
- [x] 1.10 Implement `delete_to_start()` — Cmd+Backspace
- [x] 1.11 Add private `clamp_to_char_boundary` helper and call it in every method that sets `self.cursor` or `self.anchor`

## 2. Unit tests

- [x] 2.1 Test `push_str` and cursor advancement, including multibyte characters
- [x] 2.2 Test `move_right` / `move_left` at start/end boundaries and with multibyte chars
- [x] 2.3 Test `move_word_right` / `move_word_left` across whitespace and word boundaries
- [x] 2.4 Test `move_to_start` / `move_to_end`
- [x] 2.5 Test shift-selection: anchor set on first shift-move, extended on subsequent, cleared on non-shift move
- [x] 2.6 Test `delete_before` / `delete_after` at boundaries and with multibyte chars
- [x] 2.7 Test `delete_before` with active selection deletes selected range
- [x] 2.8 Test `delete_word_before` and `delete_to_start`
- [x] 2.9 Test `selection()` returns ordered `(start, end)` regardless of cursor/anchor order

## 3. AppState migration

- [x] 3.1 Replace `AppState.input: String` with `AppState.editor: InputEditor` in `pi-oven-ui/src/lib.rs`
- [x] 3.2 Update `AppState::default()` to construct `InputEditor::default()`
- [x] 3.3 Fix all compile errors from the field rename (expect: `render_input` call, `crossterm_main`, `wgpu_main`)

## 4. render_input update

- [x] 4.1 Change `render_input` signature to accept `&InputEditor` instead of `(&str, bool)`
- [x] 4.2 Render text before cursor, then cursor cell (REVERSED if visible, plain char if not), then text after cursor
- [x] 4.3 When a selection is active, render selected region with `Modifier::REVERSED` and cursor cell within it uses the same style
- [x] 4.4 Update the `render` function in `lib.rs` to pass `&state.editor` and `state.cursor_visible`
- [x] 4.5 Verify existing layout tests still pass (`cargo test -p pi-oven-ui`)

## 5. wgpu key handler

- [x] 5.1 Replace `self.app_state.input.push_str(s)` with `self.app_state.editor.push_str(s)`
- [x] 5.2 Replace `self.app_state.input.pop()` backspace with `self.app_state.editor.delete_before()`
- [x] 5.3 Add `NamedKey::ArrowLeft` / `ArrowRight` handling: call `move_left` / `move_right` with `shift` modifier as `extend`
- [x] 5.4 Add Option+ArrowLeft / ArrowRight: call `move_word_left` / `move_word_right`
- [x] 5.5 Add Cmd+ArrowLeft / ArrowRight: call `move_to_start` / `move_to_end`
- [x] 5.6 Add `NamedKey::Delete` (forward delete): call `delete_after()`
- [x] 5.7 Add Option+Backspace: call `delete_word_before()`
- [x] 5.8 Add Cmd+Backspace: call `delete_to_start()`
- [x] 5.9 Ensure all new key actions call `reset_blink()` and `redraw()`

## 6. crossterm key handler

- [x] 6.1 Replace `app_state.input.push(c)` / `.pop()` with `editor.push_str` / `delete_before`
- [x] 6.2 Add `KeyCode::Left` / `Right` handling (no modifier support needed in crossterm for now)
- [x] 6.3 Add `KeyCode::Delete` for forward delete
- [x] 6.4 Confirm crossterm backend compiles and runs (`cargo run --no-default-features --features dev-crossterm`)
