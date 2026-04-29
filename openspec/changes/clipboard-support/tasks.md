## 1. InputEditor helpers

- [x] 1.1 Add `selected_text()` method to `InputEditor` — returns `Option<String>` containing the selected bytes, or `None` if no selection is active
- [x] 1.2 Add `delete_selection()` method to `InputEditor` — deletes the selected range and collapses the cursor to the start of the range; no-op if no selection
- [x] 1.3 Add unit tests for `selected_text()` covering: selection present, selection in reverse order, no selection
- [x] 1.4 Add unit tests for `delete_selection()` covering: selection present, no selection (no-op)

## 2. Dependency

- [x] 2.1 Add `arboard` to `crates/pi-oven/Cargo.toml` as a dependency

## 3. wgpu backend clipboard dispatch

- [x] 3.1 In `wgpu_main::handle_key`, match `KeyAction::CmdLetter('c')` — call `editor.selected_text()`, and if `Some(text)` write it to the clipboard via `arboard::Clipboard::new()?.set_text(text)`, logging a warning on error
- [x] 3.2 In `wgpu_main::handle_key`, match `KeyAction::CmdLetter('x')` — copy selected text to clipboard (same as copy), then call `editor.delete_selection()`; set `changed = true` if selection was present
- [x] 3.3 In `wgpu_main::handle_key`, match `KeyAction::CmdLetter('v')` — read text from clipboard via `arboard::Clipboard::new()?.get_text()`, on success call `editor.delete_selection()` then `editor.push_str(&text)`; set `changed = true`; log warning on clipboard error

## 4. crossterm backend clipboard dispatch

- [x] 4.1 In `crossterm_main`, match `KeyCode::Char('c')` with `KeyModifiers::CONTROL` — copy selected text to clipboard
- [x] 4.2 In `crossterm_main`, match `KeyCode::Char('x')` with `KeyModifiers::CONTROL` — cut selected text to clipboard
- [x] 4.3 In `crossterm_main`, match `KeyCode::Char('v')` with `KeyModifiers::CONTROL` — paste clipboard text at cursor
