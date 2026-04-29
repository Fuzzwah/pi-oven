## 1. Pane widget modules in `pi-oven-ui`

- [ ] 1.1 Create [crates/pi-oven-ui/src/sidebar.rs](crates/pi-oven-ui/src/sidebar.rs) exporting `pub fn render_sidebar(area: Rect, buf: &mut Buffer)`. Draw a `Block::default().borders(Borders::ALL).title("Projects")` and render the body line `(no projects)` centred horizontally one row inside the top border.
- [ ] 1.2 Create [crates/pi-oven-ui/src/tabs.rs](crates/pi-oven-ui/src/tabs.rs) exporting `pub fn render_tabs(area: Rect, buf: &mut Buffer)`. Bordered (no title), body line `(no workspaces)` left-aligned one column inside the left border.
- [ ] 1.3 Create [crates/pi-oven-ui/src/conversation.rs](crates/pi-oven-ui/src/conversation.rs) exporting `pub fn render_conversation(area: Rect, buf: &mut Buffer)`. Bordered with title `Conversation`, body line `(empty)` centred horizontally one row inside the top border.
- [ ] 1.4 Create [crates/pi-oven-ui/src/input.rs](crates/pi-oven-ui/src/input.rs) exporting `pub fn render_input(area: Rect, buf: &mut Buffer)`. Bordered (no title), body line `>` left-aligned one column inside the left border.
- [ ] 1.5 Each pane MUST handle areas smaller than its placeholder text without panic (clip the body line, but still draw the border).

## 2. Top-level layout in `pi-oven-ui`

- [ ] 2.1 In [crates/pi-oven-ui/src/lib.rs](crates/pi-oven-ui/src/lib.rs), declare `mod sidebar; mod tabs; mod conversation; mod input;` and a public `pub fn render(frame: &mut Frame)` that takes a `&mut ratatui::Frame`.
- [ ] 2.2 Inside `render`, split `frame.area()` horizontally with `Layout::horizontal([Constraint::Length(28), Constraint::Min(0)])` to produce `sidebar_area` and `right_area`.
- [ ] 2.3 Split `right_area` vertically with `Layout::vertical([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])` to produce `tabs_area`, `conversation_area`, `input_area`.
- [ ] 2.4 Call each pane's render function with its area and `frame.buffer_mut()`.
- [ ] 2.5 Re-export the four pane render functions if helpful for tests; keep the top-level `render` as the binary's only entrypoint.
- [ ] 2.6 Verify `pi-oven-ui` still has neither `pi-oven-render` nor `crossterm` in its dependency tree (`cargo tree -p pi-oven-ui --depth 1`).

## 3. Wire `pi-oven-ui::render` into the binary

- [ ] 3.1 In [crates/pi-oven/Cargo.toml](crates/pi-oven/Cargo.toml), confirm `pi-oven-ui = { path = "../pi-oven-ui" }` is listed; add it if missing.
- [ ] 3.2 In [crates/pi-oven/src/main.rs](crates/pi-oven/src/main.rs) `wgpu_main::App::redraw`, replace the inline `Paragraph::new("pi-oven")` block inside `terminal.draw(|f| { ... })` with a single call to `pi_oven_ui::render(f)`. Remove the now-unused `Rect`/`Paragraph` imports if no other code references them.
- [ ] 3.3 In `crossterm_main::run`, make the same swap inside `terminal.draw(|f| { ... })`.
- [ ] 3.4 Build both feature variants: `cargo build -p pi-oven` (default `dev-wgpu`) and `cargo build -p pi-oven --no-default-features --features dev-crossterm`. Both succeed with no warnings introduced by this change.

## 4. Unit tests with `TestBackend`

- [ ] 4.1 Create [crates/pi-oven-ui/tests/layout.rs](crates/pi-oven-ui/tests/layout.rs) (integration test). Helper: build a `Terminal::new(TestBackend::new(cols, rows))`, call `terminal.draw(|f| pi_oven_ui::render(f))`, return the rendered `Buffer`.
- [ ] 4.2 Test at 100×30: assert the sidebar's right border (column index 27) contains `│` glyphs from row 0 to row 29; assert the tabs/conversation horizontal divider sits where expected; assert the input bar's top border row index = `rows - 3`.
- [ ] 4.3 Test at 200×60: same assertions, scaled to confirm only the conversation pane absorbed the size delta (sidebar still 28 cols, input bar still 3 rows).
- [ ] 4.4 Test at 40×12 (minimum reasonable size): rendering does not panic; sidebar is still 28 cols; conversation pane is non-zero height (at least 1 row).
- [ ] 4.5 Test placeholder labels: at 100×30, assert `Projects` appears in the sidebar's title row, `(no projects)` appears in its body, `Conversation` appears in the conversation pane's title row, `(empty)` and `(no workspaces)` and `>` appear in their respective panes.

## 5. Documentation & manual verification

- [ ] 5.1 Update [README.md](README.md) Development section to note that `cargo run -p pi-oven` now opens a window with four pane shells (instead of a single `pi-oven` line).
- [ ] 5.2 Manual verification: `cargo run -p pi-oven` opens the native window; the four labelled panes are visible at the default 1280×800 size; resizing the window keeps the sidebar at 28 cols and the input/tab bars at 3 rows each; the conversation pane absorbs the change.
- [ ] 5.3 Manual verification: `cargo run -p pi-oven --no-default-features --features dev-crossterm` shows the same layout in the host terminal; pressing `q` or `Esc` exits cleanly.
- [ ] 5.4 Manual verification: `Cmd+W` still quits the wgpu build (regression check on the existing key handler).
