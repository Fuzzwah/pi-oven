# AGENTS.md

Orientation for agent sessions working on pi-oven.

## Purpose

pi-oven is a direct client/server replacement for an SSH-tunnelled TUI that wraps the [pi coding agent](https://github.com/badlogic/pi-mono/tree/main/packages/coding-agent) across git worktrees. Single user, native macOS client, Node/TS server. The project dogfoods its own intended workflow — every feature lands as an OpenSpec change.

## Status

Greenfield. One in-flight OpenSpec change: [openspec/changes/scaffold-runtime/](openspec/changes/scaffold-runtime/) — the runtime bedrock (Cargo + pnpm workspaces, server boot pipeline, native macOS window, migration runner). Nothing else has been built yet.

## Where context lives

- **Architecture plan:** [docs/claude_plan.md](docs/claude_plan.md) — the canonical roadmap with all v1 decisions, gotchas, and slice sequencing. Read first if you need the big picture.
- **In-flight change:** [openspec/changes/scaffold-runtime/](openspec/changes/scaffold-runtime/) — `proposal.md`, `design.md`, `specs/`, `tasks.md`.
- **Approved specs** (after a change archives): [openspec/specs/](openspec/specs/).
- **Skills:** [.claude/skills/](.claude/skills/) and [.pi/skills/](.pi/skills/) — the openspec slash commands.

## Tech stack

- **Client** — Rust workspace under `crates/`, split into five members so iteration is fast:
  - `pi-oven-protocol` — wire types
  - `pi-oven-render` — cell grid, `RatatuiGridBackend`, wgpu + glyphon paint, theme uniform
  - `pi-oven-ui` — ratatui widgets and layouts (Backend-trait-agnostic)
  - `pi-oven-net` — WebSocket client + reconnect/replay
  - `pi-oven` — binary; main, app shell, key dispatch, clipboard, theme loader. Packaged as a macOS `.app`. **Not a terminal program** — owning the window is the entire point, because terminal apps on macOS can't reliably see `cmd`/`option` keys.
- **Server** (`packages/pi-oven-server`) — Node 20+ / TypeScript. `better-sqlite3`, `pino`, `proper-lockfile`, `@iarna/toml`. Will embed the pi SDK in-process (`@mariozechner/pi-coding-agent`) once scaffolding lands.
- **Wire protocol** (future) — WebSocket + JSON, shared-key auth, single user.
- **Server state** — SQLite at `~/.pi-oven/state.db`. Forward-only migrations with checksum verification.
- **Server-owned filesystem** — `~/.pi-oven/` (config, lock, db + backups, logs, future event logs, future attachments).

## Dev iteration

Compile time is the dominant friction in client work; treat it as a first-class concern. Full rationale lives in [docs/claude_plan.md](docs/claude_plan.md) → "Developer iteration speed".

- **Crate split discipline.** Put new code in the most specific crate that will hold it. Putting widgets in `pi-oven-render` or types in `pi-oven-ui` defeats the split.
- **Widgets are Backend-trait-agnostic.** `pi-oven-ui` writes against `ratatui::backend::Backend`, never against `pi-oven-render` directly. The binary picks which backend to construct.
- **Feature flags on the binary** (mutually exclusive):
  - `dev-wgpu` (default) — real native rendering. Required to validate `cmd`/`option` modifier capture.
  - `dev-crossterm` — terminal-based ratatui via `CrosstermBackend`. Skips wgpu/winit startup; use for fast widget iteration. Modifier-key handling not validated under this backend.
- **Recommended dev loop:**
  ```bash
  # one terminal — fast type-check feedback on whichever crate you're editing
  cargo watch -x 'check -p pi-oven-ui'

  # another terminal — actual launches
  cargo run -p pi-oven                                                 # native window (dev-wgpu)
  cargo run -p pi-oven --no-default-features --features dev-crossterm  # terminal UI
  ```
- **Linker.** `lld` is wired up via [.cargo/config.toml](.cargo/config.toml) for the macOS targets. Requires `brew install llvm`. Saves 1-3s per incremental link.
- **Server hot reload** — `tsx watch` is on `pnpm --filter pi-oven-server dev`. Server restarts re-attach pi sessions from disk and replay buffered events to the client; restart looks like a brief disconnect from the client's perspective.
- **Don't disable the dev profile / linker config.** They're load-bearing for the iteration story.

## Workflow conventions

- **Every feature is an OpenSpec change.** Use `/opsx:propose <description>` to start one. Don't commit code outside an active change.
- **One change per slice.** Don't bundle multiple capabilities; they archive together and become hard to reason about.
- **Don't pre-write proposals for unbuilt slices.** Proposals capture *current* understanding; pre-writing produces stale fiction. Only the next slice should be in-flight.
- **Always sync specs when archiving.** When `/opsx:archive` asks about delta spec sync, always choose "Sync now". Delta specs are the source of truth for capability requirements; skipping sync leaves `openspec/specs/` stale.
- **Forward-only migrations.** Never edit a migration after it has been applied to a real DB — the runner verifies SHA-256 checksums and refuses to start on tampering. Add a *new* forward migration to fix mistakes.
- **One concern per migration file.** Rename + new column = two migrations.
- **Numbered prefixes.** `0001_initial.sql`, `0002_add_x.sql`, … lexicographic order is execution order.
- **Conventions established in `scaffold-runtime` are authoritative** once it lands. Reuse the config loader, lock, logger, and DB-open helpers; don't write parallel ones.
- **Branch off an up-to-date default.** When working in a worktree, the server (eventually) syncs the default branch before creating it; respect that contract.

## Things NOT to do

- Don't introduce code outside the workspaces. Everything lives under `crates/*` or `packages/*`.
- Don't add web/Linux/Windows clients in v1. macOS-only.
- Don't add features beyond the in-flight change's scope. New ideas → new proposal.
- Don't bypass the migration runner to `ALTER` tables. Every schema change is a migration.
- Don't remove the single-instance lock or the pre-migration backup — they exist to protect the data dir.
- Don't introduce a query builder or ORM. SQL via `better-sqlite3` is intentional.

## Useful commands

```bash
openspec status                              # all changes
openspec status --change <name>              # artifact progress
openspec instructions <artifact> --change <name>  # what each artifact should contain
openspec new change <name>                   # scaffold a new change directory
```

## Personal context to keep in mind

- **Single user.** No multi-tenancy, no per-user state, no auth surface beyond the shared key.
- **LAN/VPN deployment.** Server runs on a separate host the user reaches over their VPN. TLS is optional in v1, planned for later.
- **macOS-first, irreducibly.** The native `.app` and key handling are not a polish item — they're the reason the project exists.

## When in doubt

Read [docs/claude_plan.md](docs/claude_plan.md). If a design choice isn't covered there or in the in-flight change, ask the user before guessing.
