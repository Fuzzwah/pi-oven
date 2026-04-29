## 1. AppState — add active_workspace_id and reset defaults

- [ ] 1.1 Add `active_workspace_id: u64` field to `AppState` in `crates/pi-oven-ui/src/lib.rs`; default to `0`
- [ ] 1.2 Change `AppState::default()` tabs to an empty `Vec` (remove all four hardcoded mock `TabCell` entries)
- [ ] 1.3 Change `AppState::default()` header title to `"Connecting…"`
- [ ] 1.4 Change `AppState::default()` status fields to `"–"` for model/ctx/branch and `None` for pr

## 2. Client — Welcome handler builds tab strip

- [ ] 2.1 In `process_msg` (both wgpu_main and crossterm_main), extend the `Msg::Welcome` arm to clear `state.tabs` and rebuild from `workspaces`
- [ ] 2.2 For each `WorkspaceSnapshot`, push a `TabCell` with `idx = workspace_id as u8`, `project = format!("ws-{}", workspace_id)`, `trigger = format!("ws-{}", workspace_id)`, and `status` derived from `snapshot.status` (`"running"` → `TabStatus::Active`, else `TabStatus::Idle`)
- [ ] 2.3 Set `state.active_workspace_id` to the first workspace's `workspace_id`, or leave `0` if the list is empty
- [ ] 2.4 Set `state.header.title` to the active workspace's trigger string (or `"Connecting…"` if no active workspace)

## 3. Client — AgentStatus handler updates tab status

- [ ] 3.1 In `process_msg`, extend the `Msg::AgentStatus` arm to find the `TabCell` in `state.tabs` whose `idx == workspace_id as u8` and update its `status` to `TabStatus::Active` (running) or `TabStatus::Idle`
- [ ] 3.2 If the updated workspace matches `active_workspace_id`, also update `state.workspace_status` (existing field) as before

## 4. Layout tests — update for new defaults

- [ ] 4.1 Update `layout_100x30_tab_cells_visible` test — it currently expects hardcoded mock tab content; replace with a test that populates `state.tabs` manually before rendering
- [ ] 4.2 Update `layout_100x30_empty_tabs_placeholder` — `AppState::default()` now has empty tabs, so this test may pass trivially; verify and adjust assertion
- [ ] 4.3 Update `tabs_truncate_overflow` — already sets `state.tabs` manually, should be unaffected; verify it still passes

## 5. Smoke test

- [ ] 5.1 Start server with `PI_OVEN_SDK_STUB=1`; launch client; confirm tab strip shows `[ws-1] (ws-1)` after connecting rather than the old mock tabs
- [ ] 5.2 Type a message; confirm the tab for workspace 1 shows `Active` state while the stub is running and returns to `Idle` after
