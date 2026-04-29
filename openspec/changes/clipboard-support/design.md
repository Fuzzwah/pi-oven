## Context

`InputEditor` already has a complete selection model (`anchor` + `cursor`) and the `text()` / `selection()` accessors to read it. The wgpu path translates keyboard events through `keys.rs` into a `KeyAction` enum — `Cmd+C/V/X` will arrive as `CmdLetter('c'/'v'/'x')` and are currently ignored (fall through to `_ => false`). The crossterm path is a simpler polling loop with no modifier-aware shortcuts at all.

Clipboard I/O needs an OS API. The `arboard` crate provides a cross-platform `Clipboard` object with `get_text()` / `set_text()` — the standard choice for Rust desktop apps.

## Goals / Non-Goals

**Goals:**
- `Cmd+C` copies the selected text to the system clipboard (no-op if no selection).
- `Cmd+X` copies the selected text and deletes it from the buffer (no-op if no selection).
- `Cmd+V` inserts clipboard text at the cursor, replacing any active selection.
- Works under `dev-wgpu`. Crossterm also handled (`Ctrl+C/V/X` as terminal equivalents) on a best-effort basis.
- `InputEditor` gains two new methods (`selected_text`, `delete_selection`) to make call sites clean; no existing method signatures change.

**Non-Goals:**
- Rich-text or binary clipboard content.
- Clipboard history or multiple registers.
- Cmd+A (select-all) — not in scope for this change.

## Decisions

**Use `arboard` for clipboard access.**
`arboard` is the de-facto standard: actively maintained, wraps AppKit/Win32/X11/Wayland, no unsafe in user code. Alternatives (`copypasta`, manual FFI) offer no advantage here.

**Clipboard object created on demand per-operation, not stored on `App`.**
`arboard::Clipboard` holds an OS connection that can fail and is not `Send`. Creating it per-keystroke is negligible cost for a user-driven event. Storing it on `App` would require handling re-initialisation after errors and complicates the struct lifetime without benefit.

**New `selected_text()` and `delete_selection()` on `InputEditor` rather than inlining logic in `handle_key`.**
The selection range calculation and buffer mutation are non-trivial (byte-boundary arithmetic). Keeping them in `editor.rs` preserves the single-responsibility boundary and makes each call site read clearly (`editor.selected_text()` vs re-implementing the range lookup at each call site).

**`delete_selection()` is a no-op when there is no selection.**
Matches the semantics of every other editor operation. Call sites for cut can call `selected_text()` first, guard on `Some`, then call `delete_selection()`.

**Crossterm uses `Ctrl` modifier for clipboard shortcuts.**
Terminal emulators intercept `Cmd` on macOS. `Ctrl+C` would normally send SIGINT in raw mode — but ratatui's crossterm backend enables raw mode which suppresses that. `Ctrl+C` / `Ctrl+V` / `Ctrl+X` are the expected shortcuts in TUI editors (e.g., Helix in crossterm mode).

## Risks / Trade-offs

**`arboard` may fail to connect to the display server (Wayland/X11 headless).** → Wrap clipboard ops in a `match`; log a warning and continue on error. Never panic.

**Crossterm `Ctrl+C` conflicts with the conventional "quit" shortcut.** → The current crossterm loop uses `Esc` to quit, so there is no conflict. Document this in code comments if it becomes confusing.

**Pasting multi-line clipboard content into a single-line input bar.** → `push_str` already handles arbitrary strings; the render layer will display what fits. No special handling needed in this slice — full multi-line support is a future concern.
