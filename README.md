# pi-oven

[![License: ELv2](https://img.shields.io/badge/License-Elastic_v2-blue.svg)](LICENSE)
[![CLA](https://img.shields.io/badge/CLA-required-orange.svg)](CLA.md)

A direct client/server harness for running multiple [pi coding agent](https://github.com/badlogic/pi-mono/tree/main/packages/coding-agent) sessions in parallel across git worktrees, with a native macOS TUI client.

> **Status:** pre-alpha. The runtime is being scaffolded under [openspec/changes/scaffold-runtime/](openspec/changes/scaffold-runtime/). Nothing builds yet.

## Why use this?

If you drive a pi (or any other coding agent) over SSH from a Mac, you've probably hit some of these:

- **Hotkey theft.** macOS and your terminal eat `cmd+1`, `cmd+n` before they reach the TUI. pi-oven runs as a native `.app` that owns its window, so modifiers land where you want them.
- **One agent at a time isn't enough.** You want issue #42 in one tab, yesterday's spec in another, an exploration on a third — each isolated. pi-oven gives every workspace its own worktree, branch, and pi session, switchable with `cmd+1..9`.
- **Closing the laptop kills your work.** Agents run on the server, independent of the client. Close the lid, reopen later — the conversation pane replays the events you missed.
- **Tabs across projects all look the same.** Cycling from an agent on Project A to one on Project B with `ctrl+backquote` looks identical — a beat of "wait, which repo am I in?" every time. pi-oven assigns a theme per project (Catppuccin, Tokyo Night, Solarized, Nord, Dracula, Gruvbox, Rose Pine, Everforest, plus pi-oven defaults) and auto-switches the entire UI as you cross projects.
- **Pasting a screenshot means leaving the TUI.** Sharing a UI bug, an error panel, or a design reference today means dropping into VS Code or another editor that handles clipboard images. pi-oven takes `cmd+V` directly: text pastes inline, images are staged as attachments and sent multimodally to the agent.
- **Context-switching out of the TUI to commit, push, open PRs.** The agent does all of that as tool calls. You stay in one place from "let's start" through "ship it"; the worktree and remote branch are cleaned up automatically on merge.
- **No second set of eyes.** Every PR gets a paired **reviewer agent** in its own tab that reads the diff and posts review comments via the tracker.

It's deliberately single-user and self-hosted, leans on [OpenSpec](https://github.com/Fission-AI/OpenSpec) for feature proposals and your existing Forgejo / GitHub issues for bugs, and doesn't try to be its own task tracker.

## Why pi?

There's no shortage of coding agents. pi-oven wraps [pi](https://pi.dev) ([badlogic/pi-mono](https://github.com/badlogic/pi-mono)) specifically because:

- **Open source, MIT-licensed.** No vendor lock-in. The full agent harness — tool runner, session model, slash commands, multimodal pipeline — is auditable and forkable. If a behaviour bites and upstream is slow, you can patch it.
- **Designed to be embedded.** Pi ships an SDK (`createAgentSession`), a JSONL RPC mode (`pi --mode rpc`), and a structured event stream — all explicitly for hosting pi inside another tool. pi-oven uses the SDK in-process; no protocol reverse-engineering required.
- **Skills and prompt templates as first-class extension points.** Slash-commands like `/opsx:propose` are skill packages dropped into `.pi/skills/`. Adding domain-specific automation is editing markdown, not forking the agent.
- **Provider-agnostic.** Works with subscription LLM auth (Anthropic, OpenAI) or API keys, swap providers without rewriting your prompts. You're not betting on a single model vendor.
- **Active maintainer, focused scope.** Pi keeps a tight surface area instead of growing into a platform — easier to reason about and easier to keep up with.
- **Community ecosystem.** Skills, themes, and integrations get shared upstream; you benefit from work others do without giving up control of your own setup.

The flip side: pi is younger and smaller than the household-name agents. We pin its version exactly and treat upgrades as deliberate PRs rather than continuous-update churn.

## Stack

- **Client** — Rust native macOS app (`winit` + `wgpu` + `glyphon`, with `ratatui` via a custom backend writing into a cell grid). Packaged as a `.app` so cmd / option keys land in our event loop, not the host terminal's.
- **Server** — Node 20 + TypeScript. Embeds the pi SDK in-process, manages git worktrees, talks to Forgejo / GitHub trackers, persists state in SQLite under `~/.pi-oven/`.
- **Wire** — single WebSocket, JSON, shared-key auth.

Full architecture and rationale: [docs/claude_plan.md](docs/claude_plan.md). Agent-session orientation: [AGENTS.md](AGENTS.md).

## Workflow

Every workspace is a tab in the TUI, a worktree on disk, and a `pi` agent session. The whole point is that you can run several in parallel — each agent in its own isolated branch — without losing track of any of them.

### Starting a workspace

When you create a new workspace (`cmd+n` on a project), pi-oven first syncs the project's default branch from its remote, then asks you to pick a **trigger** that primes the agent's context:

- **Issue** — pick from open Forgejo / GitHub issues (filterable by assignee, labels, search). The branch becomes `issue-<n>-<slug>`; the agent gets the issue body and comments as initial context.
- **Spec** — pick from OpenSpec changes with incomplete tasks (sorted by remaining-task count by default). The branch becomes `spec-<change-id>`; the agent gets the change's `proposal.md` and `tasks.md`.
- **Skill** — pick from pi's available skills / prompt templates. The branch becomes `skill-<name>-<timestamp>`; the skill's prompt seeds the session.
- **Exploration** — skip everything. The branch becomes `explore-<timestamp>` and the agent starts with no priming context. On your first message, it nudges you toward `/opsx:propose` or filing an issue.

The new worktree is cut from an up-to-date default branch. If the local default can't be fast-forwarded from its remote (no remote configured, fetch failed, non-FF), pi-oven surfaces a warning in the conversation pane but still creates the worktree off the local default — work is never blocked by network or sync issues.

### Working with the agent

Inside a workspace you're talking to a normal `pi` session — slash commands, queued messages (`Enter` to steer, `Alt+Enter` to follow up, `Esc` to abort the current turn). The agent edits files in the worktree, runs tools, and commits incrementally as it goes.

`cmd+V` in the input bar handles whatever's on the clipboard: text pastes inline, an image is staged as an attachment with a thumbnail next to the input bar, then sent multimodally when you hit Enter. Multiple images per message are supported. `cmd+shift+V` forces plain-text paste if you ever want to skip image detection.

`cmd+1`…`cmd+9` jumps between tabs; `ctrl+backquote` and `ctrl+shift+backquote` cycle them (`cmd+backquote` is intercepted by macOS even in a bundled `.app`). `cmd+w` closes the focused tab. Agents run in the server, independent of your client connection: you can close your laptop, reconnect later, and the conversation pane replays everything you missed.

### Shipping the work

When the work is ready ("ship it"), the **agent itself** drives the post-work flow:

1. **Commit & push** — agent runs `git push -u origin <branch>`. HTTPS remotes use the project's tracker token via a `GIT_ASKPASS` shim; SSH remotes use the server's ssh-agent.
2. **Open PR** — agent calls the tracker (`gh` for GitHub, `tea` for Forgejo) to open a pull request targeting the default branch. Title and body come from the seed context plus a diff summary.
3. **Reviewer agent spawns** — pi-oven detects the new PR and creates a paired **review workspace** in its own tab: a separate worktree on the PR's head, a fresh `pi` session seeded with a "review the diff and post findings" prompt. The reviewer has tracker comment scope only — it cannot push or merge.
4. **You review** — read the reviewer agent's comments alongside the diff in the tracker UI. Iterate by going back to the implementation workspace and steering the agent to fix things; commit and push happen automatically; a fresh reviewer can be triggered against the new head.
5. **Merge** — configurable per project:
   - **`user-ship`** (default) — say "ship it" and the implementing agent merges its own PR. Reviewer's verdict is informational, not gating.
   - **`auto-on-approve`** — reviewer agent merges autonomously when its verdict is approve. Fastest, biggest trust delegation.
   - **`human-tracker`** — agents never merge; you click merge in the tracker UI. Use for tightly-regulated projects.
6. **Auto-cleanup** — once pi-oven sees the PR merged (webhook, polling fallback), it removes the worktree, deletes the remote branch as a safety-net, and closes both the implementation and review tabs.

### Release branches (optional, opt-in)

A project can declare a release / production branch as a **protected zone** that's locked down from agentic writes.

When a `release_branch` is configured (e.g. `production`):

- Agents **cannot push** to it.
- Agents **cannot merge** into it, regardless of `merge_mode`.
- Agents **may open** `default → release_branch` PRs so they can compose release notes / changelogs. You merge yourself in the tracker UI.

Promotion modes when a release branch is set:

- **Manual** — TUI exposes a "New release PR" affordance. Invoking it spins up a fresh release workspace where the agent opens the PR with appropriate notes. Reviewer agent spawns as usual; you click merge.
- **Auto on checks** — pi-oven watches tracker check results on `default`. When all required checks pass, the agent auto-opens the release PR for you to review. **The merge is still human-only.**
- **None** (default) — no release branch; `default` is the only mainline; step 5's `merge_mode` applies.

### Why agent-driven push and PR?

Letting the agent commit, push, open, and (optionally) merge keeps the conversational loop tight — "ship it" is the same UX as any other instruction, no context switching out of the TUI. Mitigations for the elevated trust:

- Tokens are scoped per project; one project's compromise doesn't escalate to others.
- Agents push only to their worktree's branch — never directly to `default` (always via PR), and never to `release_branch` at all.
- Reviewer agent's tracker scope is comments + approve/request-changes, plus merge-into-`default` only when `merge_mode = 'auto-on-approve'`. It cannot push code anywhere, and it cannot merge into `release_branch` even on auto-merge.
- The optional release branch is the project's hard safety net: agents may open release PRs but never merge them; that decision is always yours.
- `merge_mode = 'human-tracker'` is available per-project when you want every merge — even to `default` — to be a human action.

## Layout

```
pi-oven/
├── crates/                      # Rust workspace (TBD: scaffold-runtime)
│   ├── pi-oven/                 #   binary — native macOS app, main, keys, clipboard, themes
│   ├── pi-oven-protocol/        #   wire types shared with the server
│   ├── pi-oven-render/          #   cell grid, ratatui backend, wgpu+glyphon paint, theme
│   ├── pi-oven-ui/              #   widgets and layouts (backend-agnostic)
│   └── pi-oven-net/             #   WebSocket client, reconnect/replay
├── packages/pi-oven-server/     # Node/TS server                   (TBD: scaffold-runtime)
├── docs/claude_plan.md          # Architecture plan, decisions, gotchas
├── openspec/                    # OpenSpec changes & approved specs
├── .claude/skills/              # OpenSpec skills for Claude Code
├── .pi/skills/                  # OpenSpec skills for pi
├── AGENTS.md                    # Orientation for agent sessions
└── README.md                    # You are here
```

Splitting the client across crates is deliberate — see the *Developer iteration speed* section of [docs/claude_plan.md](docs/claude_plan.md). Editing widgets only recompiles `pi-oven-ui` + the binary, not the GPU pipeline.

## Project conventions

Every feature lands as an OpenSpec change. Bug fixes track issues on Forgejo / GitHub. From inside a `pi` or Claude session:

- `/opsx:propose <description>` — scaffold a new change (proposal, design, specs, tasks)
- `/opsx:apply` — implement the in-flight change task by task
- `/opsx:archive` — archive a completed change once its specs are merged

`openspec status` shows in-flight changes; `openspec status --change <name>` shows artifact progress.

## Development

### Prerequisites

- **Rust** stable (install via [rustup](https://rustup.rs/)).
- **Node.js** 20+.
- **pnpm** — version pinned via the root `package.json` `packageManager` field; enable with `corepack enable`.
- **`lld` linker** — `brew install llvm` provides it. The repo's [.cargo/config.toml](.cargo/config.toml) wires it in via `-fuse-ld=lld` for `aarch64-apple-darwin` and `x86_64-apple-darwin`; saves 1–3s per incremental link.
- **cargo-watch** — `cargo install cargo-watch` for the recommended tight-loop dev workflow.
- **cargo-bundle** — for producing the macOS `.app`. Install with `cargo install cargo-bundle`.

### Client crates

The client is split into five workspace members under [crates/](crates/):

- `pi-oven-protocol` — wire types shared across the UI, networking, and (eventually) server bindings.
- `pi-oven-render` — cell grid, custom `ratatui::backend::Backend`, wgpu + glyphon paint pipeline.
- `pi-oven-ui` — ratatui widgets and layouts, written against the generic `Backend` trait so they're backend-agnostic.
- `pi-oven-net` — WebSocket client and reconnect/replay logic.
- `pi-oven` — the binary; main, app shell, key dispatch, clipboard, theme loader.

The split exists so a UI tweak rebuilds one library plus the binary, not the entire wgpu pipeline.

### Client feature flags

The `pi-oven` binary exposes two mutually-exclusive feature flags:

- `dev-wgpu` (**default**) — native macOS rendering: a winit window driven by `pi-oven-render`'s wgpu/glyphon paint pipeline. Required to validate cmd / option modifier-key capture.
- `dev-crossterm` — terminal-based ratatui rendering via `ratatui::backend::CrosstermBackend`. Useful for fast iteration on widget code that doesn't need to test modifier handling.

### Client (Rust, macOS)

```bash
cargo run -p pi-oven                                                # default: native window via dev-wgpu
RUST_LOG=debug cargo run -p pi-oven                                 # verbose logging (useful for cmd/option key events)
cargo run -p pi-oven --no-default-features --features dev-crossterm # terminal-based ratatui for fast widget iteration
cargo bundle --release                                              # package as pi-oven.app under target/release/bundle/osx/
```

### Recommended dev loop

Two terminals:

```bash
# terminal 1 — fast type-check feedback on whichever crate you're editing
cargo watch -x 'check -p pi-oven-ui'

# terminal 2 — actual launches
cargo run -p pi-oven                                                 # native window
cargo run -p pi-oven --no-default-features --features dev-crossterm  # terminal UI
```

Edits to `pi-oven-ui` or `pi-oven` only recompile those two crates — the renderer (heavy GPU code) and networking crates stay cached.

### Server (Node/TS)

```bash
pnpm install                                  # install workspace deps
pnpm --filter pi-oven-server dev              # run the server in watch mode
pnpm --filter pi-oven-server test             # run vitest unit tests
pnpm --filter pi-oven-server build            # compile to dist/
pnpm --filter pi-oven-server start            # run the built server
pnpm --filter pi-oven-server migrate:status   # list applied + pending migrations
pnpm --filter pi-oven-server migrate:new <slug>   # scaffold the next-numbered migration
pnpm --filter pi-oven-server migrate:reset    # DEV-only: wipe state.db and re-run migrations
```

### State directory

The server owns `~/.pi-oven/`:

- `server.toml` — optional config; mode must be `0600` or stricter, env vars (`PI_OVEN_DATA_DIR`, `PI_OVEN_LOG_LEVEL`, `PI_OVEN_TZ`) override file values.
- `server.lock` — single-instance lock (auto-released on exit).
- `state.db` — primary SQLite database (mode `0600`).
- `state.db.bak.<unix-ms>` — automatic snapshots taken before any pending migration runs; the 10 most recent are kept.
- `logs/server-<YYYY-MM-DD>.ndjson` — daily NDJSON log files; the 7 most recent are kept.

## License

TBD.
