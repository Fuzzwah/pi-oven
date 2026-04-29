## Context

The client window currently renders a single `Paragraph` reading `pi-oven` at row 0, drawn directly from the binary in both `wgpu_main` and `crossterm_main`. The `pi-oven-ui` crate exists from scaffold-runtime but contains only a doc comment — no widgets, no layout function. Slice 1 of the roadmap (`docs/claude_plan.md`) calls for "render the same panes (empty)" before any pi SDK round-trip or workspace work begins.

The crate split is load-bearing: widgets must live in `pi-oven-ui` and be Backend-trait-agnostic so the same code paints under `dev-wgpu` (custom grid backend) and `dev-crossterm` (terminal). Putting layout in the binary or in `pi-oven-render` violates the split documented in [AGENTS.md](AGENTS.md).

## Goals / Non-Goals

**Goals:**
- Four labelled, empty pane shells visible in both backends, sized correctly at any window dimensions ≥ a minimum.
- A single public render entrypoint in `pi-oven-ui` that the binary calls inside its `terminal.draw(|f| ...)` closure.
- Unit tests using ratatui's `TestBackend` that assert the layout regions at representative sizes.
- Reuse the layout primitives ratatui already gives us; no custom geometry math.

**Non-Goals:**
- Focus model, hotkey routing, or any interactivity beyond what already exists.
- Real workspace data, real conversation events, or scrollback. Panes are stubs containing only placeholder labels.
- Theming or semantic colours — placeholder labels can use plain default styles. Semantic palette work lands with slice 6.
- Behavioural TUI baselines (scroll pinning, tab-char expansion, word-cursor input nav, wrap-not-overflow). Those are their own slice; this change only does layout.
- Any change to `pi-oven-render`, `pi-oven-net`, or the server.

## Decisions

### One render entrypoint exposed from `pi-oven-ui`

Expose a single function `pi_oven_ui::render<B: Backend>(frame: &mut Frame)` that the binary calls inside `terminal.draw(|f| ...)`. Internally it composes the four pane widgets with ratatui's `Layout`. Alternative considered: expose each widget separately and have the binary lay them out. Rejected because it would push layout decisions into the binary and duplicate them across `wgpu_main` and `crossterm_main`. A single entrypoint also means future state will be passed through it as one parameter rather than threaded through each widget call site.

In this slice the function takes no state argument. When real data arrives in later slices it will grow a parameter (e.g. `&AppState`); that's deferred until the data exists.

### Layout: fixed sidebar, fixed input bar height, conversation expands

The layout is composed with two `Layout::horizontal` / `Layout::vertical` splits:

```
┌─────────────┬──────────────────────────────────────┐
│  Projects   │  ◀ workspace tabs ▶                  │  ← height 3 (top + content + bottom)
│             ├──────────────────────────────────────┤
│  (empty)    │  Conversation                        │
│             │                                      │  ← Min(0)
│             │                                      │
│             ├──────────────────────────────────────┤
│             │  >                                   │  ← height 3
└─────────────┴──────────────────────────────────────┘
```

- Sidebar: `Constraint::Length(28)` columns. Wide enough for project names without dominating small windows; matches the screenshot in `docs/claude_plan.md`.
- Tab strip: `Constraint::Length(3)` rows (top border + label row + bottom border).
- Input bar: `Constraint::Length(3)` rows (same shape).
- Conversation: `Constraint::Min(0)` — fills the remainder.

Alternative considered: percentage-based splits. Rejected because pane chrome is character-grid sized and fixed widths/heights give predictable visual results across DPI and window sizes.

### Each pane is a `Block`-bordered widget rendering a placeholder line

Each of the four widgets is a thin `impl Widget` (or a render function — see next decision) that draws a `Block` with `Borders::ALL`, a title, and a centred placeholder line:

| Widget        | Title         | Placeholder body         |
| ------------- | ------------- | ------------------------ |
| sidebar       | `Projects`    | `(no projects)`          |
| tabs          | (none)        | `(no workspaces)`        |
| conversation  | `Conversation`| `(empty)`                |
| input         | (none)        | `>`                      |

Borders make the layout immediately legible without relying on colour. Placeholder text is intentionally bland — these slots get real rendering in later slices.

### Free functions over a `Widget` impl, for now

Each pane is a free `pub fn render_sidebar(area: Rect, buf: &mut Buffer)` (and similar for the others), called from the top-level `render` function. Alternative considered: `impl Widget for Sidebar` with a unit struct. Rejected because there is no state to hold and `Widget::render` consumes `self`, which complicates the no-arg case. When state arrives in later slices we can promote the relevant ones to `StatefulWidget`.

### Backend-trait-agnostic, with `TestBackend`-driven unit tests

Tests live in `crates/pi-oven-ui/tests/layout.rs` (or `src/lib.rs` `#[cfg(test)] mod tests`) and use `ratatui::backend::TestBackend` to render at sizes (40×12, 100×30, 200×60). Assertions:

- Each pane's border characters appear on the boundary the layout dictates.
- Titles appear at the expected coordinates.
- A minimum-size case (e.g. 40×12) does not panic and does not produce overlapping borders.

The renderer-specific tests (wgpu paint correctness) stay in `pi-oven-render` and are unaffected.

### Binary call sites mirror each other

Both `wgpu_main::App::redraw` and `crossterm_main::run` change identically: replace the inline `Paragraph::new("pi-oven")` block with `pi_oven_ui::render(f)`. This is the second time we've had two places paint the same thing; if it triples we'll factor a shared draw helper, but two is below the threshold for premature abstraction.

## Risks / Trade-offs

[Risk] **Tiny windows produce illegible output.** A 40×12 window leaves only ~9 rows × ~12 cols for the conversation pane after sidebar and chrome. → Mitigation: tests assert no panic at small sizes; visual quality at extreme sizes is accepted as a known limitation. A future "minimum window size" check is out of scope.

[Risk] **Placeholder labels become misleading once real state lands.** A user seeing `(no projects)` indefinitely in slice 1 might think the app is broken when it just isn't wired to real data yet. → Mitigation: the README development section already says the client is in scaffolding; slice 2 (multi-workspace) is the next slice and immediately replaces these labels.

[Trade-off] **No focus indicator yet.** Without highlighted-pane chrome we can't signal which pane has keyboard focus. → Accepted for this slice; focus model lands with hotkey routing in slice 2.

[Trade-off] **Free functions, not `Widget` impls.** Slightly less idiomatic for ratatui but simpler in the no-state case. Cost: when we move to `StatefulWidget` later, each pane gets a small refactor. Cheap enough.

## Migration Plan

Not applicable — no persistent state, no protocol change, no schema change. A revert is `git revert`.
