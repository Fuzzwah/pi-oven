## Why

The text input field supports cursor movement and selection, but `Cmd+C`, `Cmd+V`, and `Cmd+X` are not yet wired up — users can select text but have no way to copy, paste, or cut it. Clipboard support is the last primitive needed to make the input bar useful as a real text editor.

## What Changes

- Add `selected_text()` and `delete_selection()` helper methods to `InputEditor` to support copy/cut operations without duplicating selection logic.
- Handle `Cmd+C`, `Cmd+X`, and `Cmd+V` in `wgpu_main::handle_key` using the existing `CmdLetter` translation path in `keys.rs`.
- Integrate the `arboard` crate for cross-platform clipboard read/write in the `pi-oven` binary.
- Add clipboard handling to the `crossterm_main` event loop for parity (crossterm supports `Ctrl+C` / `Ctrl+V` / `Ctrl+X` as the terminal equivalents).

## Capabilities

### New Capabilities

- `clipboard`: clipboard read/write support for the input bar — copy selection to system clipboard on Cmd+C, cut selection on Cmd+X, paste clipboard text at cursor on Cmd+V.

### Modified Capabilities

- `client-ui`: the input bar now responds to clipboard shortcuts; no layout or rendering requirements change.

## Impact

- `crates/pi-oven/Cargo.toml` — add `arboard` dependency.
- `crates/pi-oven-ui/src/editor.rs` — new `selected_text()` and `delete_selection()` methods.
- `crates/pi-oven/src/main.rs` — `wgpu_main::handle_key` and `crossterm_main` event loop handle the new shortcuts.
- `crates/pi-oven/src/keys.rs` — no changes needed; `CmdLetter` already covers c/v/x.
