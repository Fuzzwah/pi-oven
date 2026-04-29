## 1. AppState placeholder fields

- [x] 1.1 Define `ConversationHeader { title, elapsed_secs, tokens_in, tokens_out }` in `pi-oven-ui/src/lib.rs` (or its own `state.rs` module if `lib.rs` grows too large).
- [x] 1.2 Define `StatusBar { model, ctx_pct, branch, pr: Option<u32> }`.
- [x] 1.3 Define `TabCell { idx, project, worktree, status: TabStatus, badge: Option<TabBadge> }`, `enum TabStatus { Active, Idle, Attention }`, and `enum TabBadge { Pr(u32), Unread { up: u32, down: u32 } }`.
- [x] 1.4 Add `header`, `status`, `tabs: Vec<TabCell>`, `legend: Vec<(String, String)>` fields to `AppState`.
- [x] 1.5 Populate `AppState::default()` with mock values that match the reference UX (title `Apply clipboard support changes`, elapsed `51`, tokens `(7, 2300)`, model `Sonnet 4.6`, ctx 48, branch `fuz/apply-clipboard-support`, PR `Some(9)`, two or more tabs spanning at least `Active` and `Idle`, and the legend pairs from the screenshot).
- [x] 1.6 Mark each placeholder field at construction with `// MOCK:` comments and add a one-line note at the top of each new widget file pointing readers at the wire-transport slice that will replace them.

## 2. Conversation header strip

- [x] 2.1 Create `pi-oven-ui/src/header.rs` with `pub fn render_header(area, buf, &ConversationHeader)`.
- [x] 2.2 Render the title on row 2 of the strip, left-aligned, truncated with `…` when wider than the area.
- [x] 2.3 Render the status sub-row on row 3 as `<elapsed>s · ↓<tokens_in> · ↑<tokens_out>`, formatting `tokens_out` with `k`/`M` suffixes when ≥ 1000.
- [x] 2.4 Re-export `render_header` from `lib.rs`.

## 3. Tab strip cells

- [x] 3.1 Update `pi-oven-ui/src/tabs.rs` so `render_tabs` takes `(area, buf, &[TabCell])`.
- [x] 3.2 Build a `Line` per tab cell joining: status dot (`▶` / `•` / `!`), bracketed `[idx]`, project (bold span), worktree (dim parenthesised), and an optional badge span (`#<pr>` or `↑n ↓m`).
- [x] 3.3 Pack cells left-to-right joined by a separator span; truncate the rightmost cells with `…` when total width exceeds the area.
- [x] 3.4 Preserve the existing `(no workspaces)` placeholder when `AppState.tabs` is empty.

## 4. Bottom strip (status bar + legend)

- [x] 4.1 Create `pi-oven-ui/src/status_bar.rs` with `pub fn render_status_bar(area, buf, &StatusBar)`.
- [x] 4.2 Render `<model> · ctx:<pct>% [· PR #<pr>] · <branch>` on a single row, with the PR segment present only when `pr.is_some()`, truncating the branch with `…` when the row would overflow.
- [x] 4.3 Create `pi-oven-ui/src/legend.rs` with `pub fn render_legend(area, buf, &[(String, String)])`.
- [x] 4.4 Render legend pairs joined by spaces on a single row; truncate with `…` when overflowing; do not draw a border.
- [x] 4.5 Re-export both renderers from `lib.rs`.

## 5. Layout integration

- [x] 5.1 In `pi-oven-ui/src/lib.rs::render`, replace the existing 3-strip vertical layout for the right column with a 5-strip layout: tabs (Length 3), header (Length 3), conversation body (Min 0), input (Length 3), bottom (Length 3).
- [x] 5.2 Inside the bottom strip's area, split vertically (Length 1 / Length 1 / Length 1 — top spacing, status bar, legend) and dispatch to `render_status_bar` and `render_legend`.
- [x] 5.3 Pass the new `AppState` fields into the corresponding renderers (`render_tabs(&state.tabs)`, `render_header(&state.header)`, etc.).
- [x] 5.4 Verify the binary still compiles under both `dev-wgpu` (default) and `--no-default-features --features dev-crossterm`.

## 6. Tests

- [x] 6.1 Add a `pi-oven-ui` unit test that uses `ratatui::backend::TestBackend` at a representative window size (e.g. 120×40) and asserts that the rendered buffer contains the title text, the `Sonnet 4.6` substring, the `ctx:48%` substring, and at least one tab status dot from the mock state.
- [x] 6.2 Add a small-window test (e.g. 60×20) that asserts the layout still renders without panicking and the conversation body has at least one row.
- [x] 6.3 Add a unit test that drives `render_tabs` with cells whose total width exceeds the area and asserts the trailing `…` truncation indicator is present.
- [x] 6.4 Add a unit test for the empty-tabs case that asserts the `(no workspaces)` placeholder is preserved.

## 7. Manual verification

- [ ] 7.1 Run `cargo run -p pi-oven` (default `dev-wgpu`) and confirm the right pane shows: tab cells with badges, header title + stats sub-row, `(empty)` conversation body, input bar with cursor, status bar with model/ctx/PR/branch, and the hotkey legend at the bottom.
- [ ] 7.2 Resize the window from very small to very large and confirm the chrome strips keep their fixed heights and the conversation body absorbs the change.
- [ ] 7.3 Run `cargo run -p pi-oven --no-default-features --features dev-crossterm` and confirm the same layout renders in the host terminal.
- [ ] 7.4 Confirm `Cmd+W` still quits and clipboard shortcuts (`Cmd+C/X/V`) on the input bar still behave as before.
