## Why

Before pi-oven can do anything useful, both halves of its runtime — the native macOS Rust client and the Node/TypeScript server — need a known-good starting point with the boring-but-critical plumbing already in place: process locking, structured logging, config, SQLite open with the right pragmas, a forward-only migration runner, and a native window that proves we can capture cmd/option keys. Doing this in one change means every subsequent feature builds on a runtime we already trust.

## What Changes

- Establish a Cargo workspace at the repo root with `crates/pi-oven` containing a native macOS app skeleton: a `winit` window, a `wgpu` + `glyphon` paint pipeline, a custom `ratatui::backend::Backend` writing into a cell grid, and a placeholder render that prints "pi-oven" in the window. No networking, no real UI yet.
- Package the client as a macOS `.app` via `cargo-bundle` so cmd/option keystrokes land in our event loop instead of being intercepted by a host terminal.
- Establish a pnpm workspace at the repo root with `packages/pi-oven-server` containing a Node/TS server entry point that, on startup, in this order: loads `~/.pi-oven/server.toml` (with env-var overrides), enforces `flock` on `~/.pi-oven/server.lock`, initialises pino structured logging to `~/.pi-oven/logs/server-<date>.ndjson`, opens SQLite at `~/.pi-oven/state.db` with required pragmas, and runs the forward-only migration runner. No WebSocket listener yet — boot ends with a "ready" log line.
- Ship the migration runner with checksum tracking, per-migration atomic transactions, automatic backup-before-pending-migrations via `db.backup()`, and refusal-to-start on tampered checksums or partial state. Initial migration `0001_initial.sql` creates the `_migrations` table only — feature tables come in later changes.
- Provide dev scripts: `pnpm migrate:status`, `pnpm migrate:new <slug>`, `pnpm migrate:reset` (DEV-only, typed-confirmation guarded). Provide `cargo run -p pi-oven` for the client, `pnpm --filter pi-oven-server dev` for the server.
- Provide a README documenting prerequisites, first-run config, and how to launch each side.

## Capabilities

### New Capabilities

- `server-runtime`: boot sequence, single-instance lock, config loading, structured logging, SQLite initialisation with pragmas, graceful shutdown.
- `client-runtime`: native macOS window via `winit`, wgpu/glyphon render pipeline, custom ratatui backend writing into a cell grid, macOS `.app` packaging.
- `state-migrations`: forward-only migration runner with checksum verification, per-migration atomic transactions, automatic pre-migration backups, refusal-to-start on tampering, and dev-ergonomics scripts.

### Modified Capabilities

None — this change introduces the project.

## Impact

- **New repo layout**: Cargo workspace at root (`Cargo.toml`, `crates/pi-oven/`) + pnpm workspace at root (`package.json`, `pnpm-workspace.yaml`, `packages/pi-oven-server/`).
- **New runtime dependencies**:
  - Rust client: `winit`, `wgpu`, `glyphon`, `ratatui`, `tokio`, `serde`, `serde_json`, `clap`, `tracing`, `tracing-subscriber`. Build dep: `cargo-bundle`.
  - Node server: `better-sqlite3`, `pino`, `proper-lockfile` (or native `flock` via a small native helper), `@iarna/toml` for config.
- **New filesystem footprint**: server owns `~/.pi-oven/` (`server.toml`, `server.lock`, `state.db`, `state.db.bak.<ts>`, `logs/`, `events/` placeholder, future `key`).
- **Platform**: client is macOS-only (the whole point — we need cmd/option key capture). Server runs anywhere Node 20+ runs; primary target is the user's LAN/VPN host.
- **Future changes** all assume this scaffolding: adding a feature table means a new numbered migration; adding server-side functionality plugs into the existing entry point; adding client UI plugs into the existing render loop.
