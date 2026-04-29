# Fork Inspiration

Features identified from [Fuzzwah/conduit FORK_CHANGES.md](https://github.com/Fuzzwah/conduit/blob/master/FORK_CHANGES.md) that are worth incorporating into pi-oven. These are **ideas and requirements**, not code — nothing from conduit is lifted directly.

Features already covered by the current plan (image paste → Slice 3, themes → Slice 6, issue picker → Slice 4) and features not applicable to pi-oven's architecture (multi-agent model, web UI, tmux) are omitted.

---

## Slice 1 Baseline Requirements

These should be correctness requirements written into the Slice 1 spec, not left as polish.

### Scroll Position Preserved During Streaming
While the agent is generating output, scrolling up pins the viewport to your position. New content growing below does not yank the view down. Scrolling back to the bottom restores auto-follow.

### Tab Characters Expanded in Tool Output
Tab characters in tool output are expanded to 8-column tab stops before rendering, matching standard terminal behaviour. Without this, Read tool output (`line_number\TABcode`) collapses line numbers into code.

### TUI Text Input Correctness
- `Option+Left` / `Option+Right` (which send `Alt+b` / `Alt+f` escape sequences) move cursor backward / forward by word in the chat input.
- Long text in the input wraps within the box rather than overflowing off-screen.
- Continuation rows indent consistently (no 2-character misalignment on wrapped lines).

---

## Slice 2 — Multi-Workspace + Tabs

### Ahead/Behind Counts in Sidebar
Each workspace in the sidebar shows `↑N` (yellow) and `↓N` (red) when its branch has commits ahead of or behind `origin/<default>`. Both suppressed when zero. Refreshed every 30 seconds without a network fetch — use cached local git state.

```
  feature/my-branch  ↑2 ↓1
```

### Pinned Agent Status Message
The latest assistant status message is pinned to the top of the chat viewport once tool output pushes it out of view. A `─` separator marks the boundary between the pinned message and scrollable content below. The pin deactivates automatically when the message is still within the normal scroll view.

### Sidebar Selection Tracks Active Tab
When switching or closing tabs with the sidebar hidden, the sidebar selection stays in sync. Opening the sidebar after a tab switch highlights the correct workspace without requiring user action.

### Backward Tab Cycling (`cmd+Shift+\``)
A reverse-direction companion to the existing forward tab cycle shortcut. Footer hint updated to show both.

### Copy Code Blocks to Clipboard (`cmd+y`)
`cmd+y` copies the nearest visible code block to the clipboard. Pressing repeatedly cycles through all code blocks visible in the current output. Integrates with the existing native clipboard infrastructure.

### `/btw` — Queue a Note Without Interrupting
`/btw <note>` queues the note as a follow-up message without interrupting the running agent. Works whether the agent is idle or actively generating. `/btw` with no args opens a queue editor. Appears in the `/` autocomplete menu.

### `@filename` Autocomplete in Chat Input
Typing `@` in the chat input triggers a file autocomplete menu listing files in the current workspace worktree. Selecting a file inserts its path as a mention. Fuzzy-filtered as the user types.

### Always-Visible Sidebar Config
```toml
[ui]
always_show_sidebar = true
```
When enabled, the sidebar toggle shortcut moves focus to/from the sidebar rather than hiding it. Opening or creating a workspace does not close the sidebar. `Escape` returns focus to chat input while keeping the sidebar visible.

---

## Slice 3 — Image Attachments

### SCP Upload to Workspace (`cmd+u`)
Since pi-oven's server runs on a separate LAN machine, SCP is the natural upload path for large files. The flow:

1. `cmd+u` opens a destination browser within the current workspace worktree.
2. User selects a directory and presses `c` to confirm.
3. pi-oven displays the SCP command with the absolute server-side destination path and copies it to the clipboard via OSC 52 (works over SSH and tmux).
4. User runs the SCP command from their Mac.
5. User presses Enter inside pi-oven; it scans the destination for newly arrived or updated files and reports their names.

This complements the clipboard image paste flow: paste for screenshots/diagrams, SCP for larger files.

---

## Slice 5 — Add-Project Flow

### Workspace Setup Script
After creating a worktree, if a `workspace_setup.sh` exists in the repository root, run it automatically. Useful for installing dependencies, setting up `.env` files, or any other per-workspace initialisation. Logged to the workspace event log.

### Error Dialog for Duplicate Directory
When adding a project via git URL, if the derived target directory already exists in the projects base directory, show an inline error (`Directory '<name>' already exists`) instead of silently failing.

---

## Slice 7 — Tracker Observability + Merge Cleanup

### Squash-Merge Detection in Archive Preflight
When a workspace's archive preflight checks whether its branch is merged, distinguish between genuinely unmerged branches and squash-merged ones. If the branch has commits not in main's ancestry but the diff against main is empty, show "Squash-merged (N commits ahead, diff already in main)" at informational severity rather than the alarming "Branch not merged" warning.

Optional `gh` CLI integration for live PR state:
```toml
[workspaces]
use_gh_cli_merge_status = true
```
When set, the archive dialog shows: "PR merged (via GitHub)", "PR is open", "PR is a draft", or "PR closed without merging" (as a warning). Git-based detection remains active as a fallback.

---

## Slice 10 — Polish

### Quit Confirmation Dialog
`Ctrl+Q` opens a confirmation dialog with **Quit** (pre-selected) and **Cancel**. Pressing Enter immediately after `Ctrl+Q` quits. `Escape` dismisses. `Ctrl+Q` a second time while the dialog is open also confirms. Prevents accidental quit during active agent runs.

### Archive Workspace from Inside a Tab (`cmd+Shift+X`)
Consistent archive shortcut available both from within a workspace tab and from the sidebar, matching the sidebar's existing `x` action. Archive hint appears in the chat footer. Available in the command palette.

### Reorder Projects in Sidebar
Move-up / move-down keyboard actions for project order in the sidebar. Order persisted in the server DB. Reflects immediately across all connected clients.

### Context-Window Percentage (`ctx%`)
Accurate `ctx%` counter based on per-call token usage (`input + cache_creation + cache_read`) from each assistant message, not cumulative totals. Drops naturally after compaction. Display in the chat pane status bar.

### Copy Local File into Workspace (`cmd+a`)
Two-step file browser for copying a local Mac file into the current workspace worktree via the client:

1. Browse the local Mac filesystem to select a source file.
2. Browse the workspace repository tree; press `c` to confirm the destination.

File is transferred from client to server over the WebSocket connection (reuses the attachment upload framing from Slice 3). A footer message confirms the destination on success; `Escape` cancels at any point.

---

## Excluded / Not Applicable

| Conduit # | Feature | Reason excluded |
|---|---|---|
| 3 | Alt+Tab sidebar exclusion | pi-oven uses cmd-based keys, not Alt |
| 4 | Plan mode input wrap | conduit-specific "plan mode" UI |
| 5 | TUI auto-refresh on web UI change | pi-oven has no web UI; WebSocket handles this natively |
| 15 | Full plan content in chat | conduit-specific plan review step |
| 16 | GitHub Copilot as agent | pi-oven is pi-specific |
| 17 | tmux config | pi-oven is a native macOS .app |
| 25 | Web chat auto-follow | no web UI in v1 |
| 27 | Pi as agent | pi-oven IS the pi client |
| 29 | Dirac as agent | pi-oven is pi-specific |
| 30 | Provider selector re-detect | no multi-provider model |
| 31 | OpenCode tool display | conduit-specific |
| 34 | Copilot model IDs | not applicable |
