## 1. InputEditor struct

- [ ] 1.1 Create `crates/pi-oven-ui/src/editor.rs` with `InputEditor` struct (fields: `buf: String`, `cursor: usize`, `anchor: Option<usize>`) and `pub use` from `lib.rs`
- [ ] 1.2 Implement `Default`, `text()`, `cursor_byte_pos()`, `selection()` (returns ordered `Option<(usize, usize)>`)
- [ ] 1.3 Implement `push_str(s: &str)` — insert at cursor, advance cursor
- [ ] 1.4 Implement `move_right(extend: bool)` and `move_left(extend: bool)` with selection-collapse logic
- [ ] 1.5 Implement `move_word_right(extend: bool)` and `move_word_left(extend: bool)` using whitespace word boundaries
- [ ] 1.6 Implement `move_to_start(extend: bool)` and `move_to_end(extend: bool)`
- [ ] 1.7 Implement `delete_before()` — backspace; deletes selection if active
- [ ] 1.8 Implement `delete_after()` — forward delete
- [ ] 1.9 Implement `delete_word_before()` — Option+Backspace
- [ ] 1.10 Implement `delete_to_start()` — Cmd+Backspace
- [ ] 1.11 Add private `clamp_to_char_boundary` helper and call it in every method that sets `self.cursor` or `self.anchor`

## 2. Unit tests

- [ ] 2.1 Test `push_str` and cursor advancement, including multibyte characters
- [ ] 2.2 Test `move_right` / `move_left` at start/end boundaries and with multibyte chars
- [ ] 2.3 Test `move_word_right` / `move_word_left` across whitespace and word boundaries
- [ ] 2.4 Test `move_to_start` / `move_to_end`
- [ ] 2.5 Test shift-selection: anchor set on first shift-move, extended on subsequent, cleared on non-shift move
- [ ] 2.6 Test `delete_before` / `delete_after` at boundaries and with multibyte chars
- [ ] 2.7 Test `delete_before` with active selection deletes selected range
- [ ] 2.8 Test `delete_word_before` and `delete_to_start`
- [ ] 2.9 Test `selection()` returns ordered `(start, end)` regardless of cursor/anchor order

## 3. AppState migration

- [ ] 3.1 Replace `AppState.input: String` with `AppState.editor: InputEditor` in `pi-oven-ui/src/lib.rs`
- [ ] 3.2 Update `AppState::default()` to construct `InputEditor::default()`
- [ ] 3.3 Fix all compile errors from the field rename (expect: `render_input` call, `crossterm_main`, `wgpu_main`)

## 4. render_input update

- [ ] 4.1 Change `render_input` signature to accept `&InputEditor` instead of `(&str, bool)`
- [ ] 4.2 Render text before cursor, then cursor cell (REVERSED if visible, plain char if not), then text after cursor
- [ ] 4.3 When a selection is active, render selected region with `Modifier::REVERSED` and cursor cell within it uses the same style
- [ ] 4.4 Update the `render` function in `lib.rs` to pass `&state.editor` and `state.cursor_visible`
- [ ] 4.5 Verify existing layout tests still pass (`cargo test -p pi-oven-ui`)

## 5. wgpu key handler

- [ ] 5.1 Replace `self.app_state.input.push_str(s)` with `self.app_state.editor.push_str(s)`
- [ ] 5.2 Replace `self.app_state.input.pop()` backspace with `self.app_state.editor.delete_before()`
- [ ] 5.3 Add `NamedKey::ArrowLeft` / `ArrowRight` handling: call `move_left` / `move_right` with `shift` modifier as `extend`
- [ ] 5.4 Add Option+ArrowLeft / ArrowRight: call `move_word_left` / `move_word_right`
- [ ] 5.5 Add Cmd+ArrowLeft / ArrowRight: call `move_to_start` / `move_to_end`
- [ ] 5.6 Add `NamedKey::Delete` (forward delete): call `delete_after()`
- [ ] 5.7 Add Option+Backspace: call `delete_word_before()`
- [ ] 5.8 Add Cmd+Backspace: call `delete_to_start()`
- [ ] 5.9 Ensure all new key actions call `reset_blink()` and `redraw()`

## 6. crossterm key handler

- [ ] 6.1 Replace `app_state.input.push(c)` / `.pop()` with `editor.push_str` / `delete_before`
- [ ] 6.2 Add `KeyCode::Left` / `Right` handling (no modifier support needed in crossterm for now)
- [ ] 6.3 Add `KeyCode::Delete` for forward delete
- [ ] 6.4 Confirm crossterm backend compiles and runs (`cargo run --no-default-features --features dev-crossterm`)
