## Context

The right pane currently has three rendered strips (tabs at top, conversation in the middle, input at the bottom) plus a fixed-width sidebar on the left. The reference UX adds two more strips (header above conversation, status + legend below input), and turns the bare tab strip into a list of tab cells with badges. The change is purely visual — there is no event source, no focus model, and no wire data feeding any of it. The job of this design is to settle two things: where the new state fields live on `AppState`, and how the layout grows without disturbing the load-bearing scenarios that the existing `client-ui` spec pins down (sidebar 28 cols, tabs top 3 rows, input bottom 3 rows, conversation absorbs).

## Goals / Non-Goals

**Goals:**
- Land all four pieces of static chrome (header, tab cells, status bar, legend) in one slice so the silhouette of the right pane matches the target.
- Keep `pi-oven-ui` Backend-trait-agnostic — the new widgets paint under both `dev-wgpu` and `dev-crossterm` without conditional code.
- Keep the existing four-pane scenarios in `client-ui` working (sidebar width, tab strip top, input bottom, conversation absorbs the remainder); do not regress any archived requirement.
- Make the placeholder data trivial to swap for real state later — fields on `AppState` should look like what real wire data will look like.

**Non-Goals:**
- No focus model, no hotkey routing for the new chrome. The legend is purely descriptive text.
- No real workspace/tab/conversation data. Every value is a hard-coded placeholder in `AppState::default()`.
- No sidebar redesign. Sidebar keeps its current `Projects` / `(no projects)` placeholder.
- No conversation body changes. The `(empty)` body stays.
- No new dependencies. All chrome is plain ratatui widgets.

## Decisions

### Decision 1 — Layout: four vertical strips on the right, in a fixed order

**Chosen:** the right column becomes a 5-row vertical layout: `tabs (3)` · `header (3)` · `conversation (Min 0)` · `input (3)` · `status+legend (3)`. The bottom 3-row strip is itself split horizontally-then-vertically inside the same widget into a 1-row status bar above a 1-row legend, with one row of padding/border accounted for inside the box.

**Why:** the `client-ui` spec pins the tab strip to "the top 3 rows of the right column" and the input bar to "the bottom 3 rows" — past tense applied to *the right column*, not the screen. Adding strips above/below those panes inside the same column is consistent with the spec's wording, but the existing scenarios for "tab strip pinned to top" and "input bar pinned to bottom of right column" need to be re-stated against the new neighbours (header below tabs; status above input bar's bottom edge). We update those scenarios in the delta rather than relocating the tab/input panes.

**Alternative considered:** Put the status bar + legend *outside* the right column (i.e. across the full window width, including under the sidebar). Rejected because the sidebar would float disconnected from a visible bottom edge, and because the reference screenshot shows the legend spanning the full width — but for v1, the simpler decision is to keep everything within the right column and let the sidebar terminate at its own bottom border. Cross-window-width strips can come later when the sidebar grows real content.

### Decision 2 — `AppState` placeholder fields

**Chosen:** add four new fields:
- `header: ConversationHeader { title: String, elapsed_secs: u64, tokens_in: u64, tokens_out: u64 }`
- `status: StatusBar { model: String, ctx_pct: u8, branch: String, pr: Option<u32> }`
- `tabs: Vec<TabCell { idx: u8, project: String, worktree: String, status: TabStatus, badge: Option<TabBadge> }>` where `TabStatus` is `Active | Idle | Attention` and `TabBadge` is `Pr(u32) | Unread(u32)`
- `legend: Vec<(String /*keys*/, String /*action*/)>` — pre-built in `AppState::default()` from a const list

**Why:** typed structs (rather than raw strings) are a tiny up-front cost and pay off when the wire transport eventually feeds real values — the field names and shapes already match. `legend` could be a const, but storing it on `AppState` means the eventual focus-model slice can swap entries based on which pane is focused without restructuring the renderer.

**Alternative considered:** a single `MockData` struct holding everything. Rejected — when the real data wires in, swapping a typed field for a real source is a one-line change; swapping fields out of a `MockData` blob requires rewiring every reader.

### Decision 3 — One widget module per strip

**Chosen:** four new files in `pi-oven-ui/src/`: `header.rs`, `status_bar.rs`, `legend.rs`, plus a rewrite of `tabs.rs`. Each exposes a single `render_*` free function taking `(area, buf, &state_field)`, mirroring the pattern already used by `sidebar.rs` / `conversation.rs` / `input.rs`.

**Why:** consistency with the existing widget conventions. Free functions over structs avoid a second layer of state, and each module stays small enough that splitting later is cheap if any one of them grows complex.

### Decision 4 — Tab cell rendering

**Chosen:** `tabs.rs` builds a single `Line` of styled `Span`s per tab cell, joined left-to-right by a separator span. Cells render: status dot (`▶` active, `•` idle, `!` attention) + `[N]` index + project name (bold) + worktree name (dim parenthesised) + optional badge (`#NN` for PR, `↑n ↓m` for unread counts) styled in a colour that maps to the badge type. Overflow: when total cell width exceeds available columns, the rightmost cells are truncated with `…`.

**Why:** the reference screenshot shows tabs as horizontally packed pills with a clear primary/secondary text split. A single `Line` per row is the cheapest path under ratatui and keeps the strip a fixed 3 rows. Truncation at the right edge mirrors how the existing input bar handles overflow.

**Alternative considered:** ratatui's built-in `Tabs` widget. Rejected — it doesn't support multi-text-style cells (dot + bold + dim + badge). Rolling our own keeps full control.

### Decision 5 — Status + legend share a 3-row bottom strip

**Chosen:** the bottom 3 rows below the input bar render as one widget that internally lays out: row 1 = top border / spacing, row 2 = status bar, row 3 = legend. No outer border around the legend; the status bar uses subtle separator dots (`·`) between segments.

**Why:** combining them into one widget keeps the layout call in `lib.rs` symmetric with the top (tabs + header are also one logical unit). Two adjacent borders here would look heavy; suppressing the legend's border and using a single shared frame reads cleaner against the input bar's bottom edge.

## Risks / Trade-offs

- **Risk:** adding two strips to the right column tightens the conversation area on small windows.
  **Mitigation:** the four fixed strips total 12 rows of vertical chrome (tabs 3 + header 3 + input 3 + status/legend 3); on the smallest realistic window (~24 rows) the conversation pane still gets ~12 rows. We document the minimum window size in the spec.

- **Risk:** the existing `client-ui` "Tab strip pinned to top of right column" and "Input bar pinned to bottom of right column" scenarios become technically true but misleading once new strips sit immediately below/above them. Future readers might assume those panes still touch the column edges.
  **Mitigation:** restate the relevant scenarios in the delta spec to make the new neighbours explicit (e.g. "the tab strip occupies the top 3 rows" → "the tab strip occupies rows 1–3 of the right column; the conversation header occupies rows 4–6").

- **Risk:** the placeholder data on `AppState::default()` makes the app look like it has real state. A reader picking up the project might reasonably assume the wire transport is feeding it.
  **Mitigation:** add a `// MOCK:` comment on each field in `AppState::default()` and a one-line note at the top of the relevant widget files explaining the demo source. The eventual wire-transport slice removes those comments as it removes the placeholder defaults.

- **Trade-off:** picking typed structs over plain strings for placeholder fields is over-engineering relative to v1's needs. We accept the cost because every one of these fields is going to be replaced by a real wire-typed struct within a couple of slices.

- **Trade-off:** legend strings are duplicated between the legend renderer and any future hotkey handler. We accept the duplication for v1; a shared `KeyBinding` registry can come once a real handler exists to consume it.
