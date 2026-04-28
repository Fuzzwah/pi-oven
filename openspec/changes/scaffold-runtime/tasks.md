## 1. Repo skeleton

- [ ] 1.1 Add root `Cargo.toml` declaring a workspace with `members = ["crates/*"]` and `resolver = "2"`
- [ ] 1.2 Add root `package.json` with `"private": true` and `"packageManager"` pinned to a specific pnpm version
- [ ] 1.3 Add `pnpm-workspace.yaml` with `packages: - "packages/*"`
- [ ] 1.4 Add `.gitignore` covering Rust (`target/`), Node (`node_modules/`, `dist/`), and macOS (`.DS_Store`)
- [ ] 1.5 Update [README.md](README.md) with prerequisites (Rust stable, Node 20+, pnpm), per-side dev commands, and where state lives (`~/.pi-oven/`)

## 2. Server scaffolding

- [ ] 2.1 Create `packages/pi-oven-server/package.json` pinning `better-sqlite3`, `pino`, `pino-pretty`, `proper-lockfile`, `@iarna/toml`, and dev deps `typescript`, `tsx`, `vitest`
- [ ] 2.2 Create `packages/pi-oven-server/tsconfig.json` (strict, ESM, target ES2022, `moduleResolution: bundler`)
- [ ] 2.3 Add npm scripts: `dev` (tsx watch on `src/index.ts`), `build` (tsc), `start` (run built JS), `migrate:status`, `migrate:new`, `migrate:reset`, `test` (vitest)

## 3. Server config loader

- [ ] 3.1 Implement [packages/pi-oven-server/src/config.ts](packages/pi-oven-server/src/config.ts) with `loadConfig()` returning `{ data_dir, log_level, tz }`
- [ ] 3.2 Read `~/.pi-oven/server.toml` if present using `@iarna/toml`; fall back to defaults if missing
- [ ] 3.3 Apply env-var overrides for `PI_OVEN_DATA_DIR`, `PI_OVEN_LOG_LEVEL`, `PI_OVEN_TZ`
- [ ] 3.4 Stat the config file, refuse to start if mode is looser than `0600` (use `fs.statSync` + `mode & 0o077`)
- [ ] 3.5 Resolve `data_dir` using `os.homedir()` if it begins with `~`
- [ ] 3.6 Vitest tests covering: defaults when file missing, file values, env override, permission refusal

## 4. Single-instance lock

- [ ] 4.1 Implement [packages/pi-oven-server/src/lock.ts](packages/pi-oven-server/src/lock.ts) exposing `acquireLock(dir): Promise<Release>` using `proper-lockfile`
- [ ] 4.2 On acquire, write `{ pid, started_at }` JSON into `<dir>/server.lock` body for diagnostics
- [ ] 4.3 On EWOULDBLOCK / locked: read existing body, throw a typed error including the holding PID
- [ ] 4.4 Register a `process.on('exit'|'SIGINT'|'SIGTERM')` handler that releases the lock
- [ ] 4.5 Vitest test: spawning two processes — second exits non-zero with the first's PID in stderr

## 5. Structured logging

- [ ] 5.1 Implement [packages/pi-oven-server/src/log.ts](packages/pi-oven-server/src/log.ts) creating a pino root logger
- [ ] 5.2 Configure file destination at `<data_dir>/logs/server-<YYYY-MM-DD>.ndjson` using pino's transport (or `pino.destination`) — date is computed in the configured TZ
- [ ] 5.3 If `NODE_ENV=development`, attach a `pino-pretty` transport for stdout in addition to the file
- [ ] 5.4 On logger init, scan `<data_dir>/logs/` and delete `server-*.ndjson` files older than the 7 most recent
- [ ] 5.5 Export a `childLogger(bindings)` helper for adding `workspace_id` and similar correlation fields later
- [ ] 5.6 Vitest test: logged line is valid JSON with `level`, `time`, `pid`, `msg`; old-log pruning leaves exactly 7 files

## 6. Migration runner

- [ ] 6.1 Implement [packages/pi-oven-server/src/state/migrate.ts](packages/pi-oven-server/src/state/migrate.ts) exposing `migrate(db, migrationsDir): Promise<{ applied: string[] }>`
- [ ] 6.2 Create the `_migrations` table if missing (idempotent `CREATE TABLE IF NOT EXISTS`)
- [ ] 6.3 List applied migrations from `_migrations`; list files in `migrationsDir` matching `^\d{4}_.*\.(sql|ts)$`; sort lexicographically
- [ ] 6.4 For each applied row: verify the corresponding file exists and its SHA-256 matches the recorded checksum; if not, throw with a descriptive message
- [ ] 6.5 Compute pending = files whose names are not in `_migrations`
- [ ] 6.6 If pending is non-empty: call `db.backup('<state.db path>.bak.<unix-ms>')` and await its `Promise`
- [ ] 6.7 For each pending file in order: open `BEGIN IMMEDIATE`, run the migration (`db.exec(text)` for `.sql`, dynamic `import()` and call `up(db)` for `.ts`), insert the `_migrations` row, `COMMIT`. On error: `ROLLBACK` and rethrow
- [ ] 6.8 After successful run, prune backups beyond the 10 most recent
- [ ] 6.9 Vitest tests covering: fresh DB → all migrations apply; current DB → no-op (no backup); partial → only remaining apply; tampered checksum → throws and DB unchanged; throwing migration → transaction rolls back, `_migrations` unchanged

## 7. Initial migration and database open

- [ ] 7.1 Create `packages/pi-oven-server/src/state/migrations/0001_initial.sql` containing only the `_migrations` table DDL (matches the `state-migrations` spec)
- [ ] 7.2 Implement [packages/pi-oven-server/src/state/db.ts](packages/pi-oven-server/src/state/db.ts) exposing `openDb(path): Database` that creates the file with mode `0600` if missing and applies pragmas in the documented order
- [ ] 7.3 Vitest test: opening a fresh path produces a file with mode `0600` and the expected pragma values via `db.pragma('...')`

## 8. Server entry point and boot sequence

- [ ] 8.1 Implement [packages/pi-oven-server/src/index.ts](packages/pi-oven-server/src/index.ts) that, on startup, in order: `loadConfig` → `acquireLock` → `initLogger` → `openDb` → `migrate(db, migrationsDir)` → log `"ready"` with `{ pid, version, data_dir }`
- [ ] 8.2 Wrap each step in a try/catch; on failure, log an `error` line naming the failed step and exit with status `1`
- [ ] 8.3 Register `SIGINT`/`SIGTERM` handlers that flush logs (`logger.flush()`), close the DB (`db.close()`), release the lock, and exit zero
- [ ] 8.4 Manual verification: `pnpm --filter pi-oven-server dev` produces a `"ready"` log line and stays running until Ctrl+C; running it twice in parallel makes the second instance exit non-zero with the first's PID

## 9. Migration management scripts

- [ ] 9.1 Implement `scripts/migrate-status.ts` printing applied (name, applied_at) and pending (name) lists
- [ ] 9.2 Implement `scripts/migrate-new.ts <slug>` scaffolding `<NNNN>_<slug>.sql` (next number, kebab-case slug normalisation)
- [ ] 9.3 Implement `scripts/migrate-reset.ts` requiring `NODE_ENV !== 'production'` AND a typed confirmation matching the data directory path; deletes `state.db` (and `state.db-wal`/`-shm`) then runs `migrate()`
- [ ] 9.4 Wire each script as the body of the matching `pnpm migrate:*` command in `package.json`

## 10. Client multi-crate workspace structure

Lock in the multi-crate split before any client code lands — see design D11 and the **Developer iteration speed** section of [docs/claude_plan.md](../../../docs/claude_plan.md). Each library crate ships with a stub `lib.rs` so the workspace compiles end-to-end before real implementations land in later sections.

- [ ] 10.1 Confirm the existing `members = ["crates/*"]` glob in root [Cargo.toml](Cargo.toml) (from task 1.1) automatically picks up the new sub-crates created below — no workspace-members update should be required
- [ ] 10.2 Create `crates/pi-oven-protocol/Cargo.toml` with deps `serde`, `serde_json`; stub `src/lib.rs` (empty `Msg` placeholder)
- [ ] 10.3 Create `crates/pi-oven-render/Cargo.toml` with deps `ratatui` (default-features = false), `wgpu`, `glyphon`, `image`, `bytemuck`, `winit` (for surface integration); stub `src/lib.rs`
- [ ] 10.4 Create `crates/pi-oven-ui/Cargo.toml` with deps `ratatui` (default-features = false) and `pi-oven-protocol` (path dep); stub `src/lib.rs`. **Does NOT depend on `pi-oven-render`** — widgets are Backend-trait-agnostic.
- [ ] 10.5 Create `crates/pi-oven-net/Cargo.toml` with deps `tokio`, `tokio-tungstenite`, `pi-oven-protocol` (path dep), `tracing`; stub `src/lib.rs`
- [ ] 10.6 Create `crates/pi-oven/Cargo.toml` (binary) with deps on all four library crates plus `winit`, `clap`, `tokio` (rt + macros), `arboard`, `tracing`, `tracing-subscriber`, `anyhow`; `[package.metadata.bundle]` declaring identifier `dev.fuzzwah.pi-oven`, name `pi-oven`, category `public.app-category.developer-tools`, placeholder icon path
- [ ] 10.7 Define feature flags on `pi-oven` binary in its `Cargo.toml`: `dev-wgpu` (default, enables `pi-oven-render` integration), `dev-crossterm` (mutually exclusive, uses `ratatui::backend::CrosstermBackend` from `ratatui` directly). Document both in README.
- [ ] 10.8 Stub [crates/pi-oven/src/main.rs](crates/pi-oven/src/main.rs) that initialises `tracing_subscriber::fmt`, prints "pi-oven scaffolded" at info level, and exits cleanly. No window yet.
- [ ] 10.9 Run `cargo build --workspace` and confirm all five crates compile. Run `cargo run -p pi-oven` and confirm the stub message appears.

## 11. Developer iteration tooling

Set up the fast-iteration scaffolding before the heavy renderer/widget code lands.

- [ ] 11.1 Add the dev profile to root [Cargo.toml](Cargo.toml): `opt-level = 0`, `debug = "line-tables-only"`, `codegen-units = 256`, `incremental = true` for `[profile.dev]`; `opt-level = 0`, `debug = false` for `[profile.dev.package."*"]`
- [ ] 11.2 Create [.cargo/config.toml](.cargo/config.toml) with `lld` linker config for `aarch64-apple-darwin` and `x86_64-apple-darwin` (`linker = "clang"`, `rustflags = ["-C", "link-arg=-fuse-ld=lld"]`)
- [ ] 11.3 Update README's Development section: prerequisites now include `brew install llvm` for `lld`, `cargo install cargo-watch` for the recommended dev loop, and document the `dev-crossterm` / `dev-wgpu` feature flags
- [ ] 11.4 Document the recommended dev loop in README: `cargo watch -x check -p <crate>` in one terminal for fast feedback, `cargo run -p pi-oven` in another for actual launches; `cargo run -p pi-oven --no-default-features --features dev-crossterm` for terminal-based UI iteration
- [ ] 11.5 Verify the dev profile takes effect: `cargo build -p pi-oven` in dev mode finishes a clean build in a reasonable time; an incremental rebuild after touching `pi-oven-ui/src/lib.rs` only recompiles `pi-oven-ui` and `pi-oven` (not the renderer or networking crates) — confirm via `cargo build -p pi-oven --timings` output
- [ ] 11.6 Verify `lld` is being used: `cargo build -p pi-oven -v 2>&1 | grep fuse-ld` returns the link-arg

## 12. Cell grid and ratatui backend

- [ ] 12.1 Implement [crates/pi-oven-render/src/grid.rs](crates/pi-oven-render/src/grid.rs) defining `Cell { ch: char, fg: Color, bg: Color, attrs: Attrs }` and `Grid { cols, rows, cells: Vec<Cell> }` with `resize(cols, rows)` and `set(x, y, cell)`
- [ ] 12.2 Implement [crates/pi-oven-render/src/backend.rs](crates/pi-oven-render/src/backend.rs) as `RatatuiGridBackend` implementing `ratatui::backend::Backend` by writing the diff into the `Grid`
- [ ] 12.3 Re-export `Grid` and `RatatuiGridBackend` from `pi-oven-render`'s `lib.rs` so the binary can construct them
- [ ] 12.4 In [crates/pi-oven/src/main.rs](crates/pi-oven/src/main.rs) (under `#[cfg(feature = "dev-wgpu")]` or default), wire a draw step that lays out a `Paragraph::new("pi-oven")` at `Rect::new(0, 0, cols, 1)` via `RatatuiGridBackend` and writes it into the grid as the first frame's content
- [ ] 12.5 Unit test the backend in `pi-oven-render/tests/`: a small `Terminal::draw` call yields the expected cells in the grid

## 13. wgpu + glyphon paint pipeline

- [ ] 13.1 Implement [crates/pi-oven-render/src/paint.rs](crates/pi-oven-render/src/paint.rs) constructing a wgpu `Instance`, `Surface`, `Adapter`, `Device`, `Queue` for a winit window passed in by the binary
- [ ] 13.2 Set up a `glyphon::FontSystem`, `SwashCache`, and `TextAtlas`; load a bundled monospace font (e.g. JetBrains Mono Nerd Font subset) under `crates/pi-oven-render/assets/fonts/`
- [ ] 13.3 Build a `glyphon::Buffer` per frame from the cell grid: one styled run per contiguous same-style cell range
- [ ] 13.4 Render the buffer to the surface; clear to a configured background colour first
- [ ] 13.5 Handle `WindowEvent::Resized` and `ScaleFactorChanged`: reconfigure the surface and recompute grid dimensions from window size and font metrics

## 14. Native window and key handling

- [ ] 14.1 Replace the stub [crates/pi-oven/src/main.rs](crates/pi-oven/src/main.rs) with a winit window opening flow: title `pi-oven`, default size (e.g. 1280x800)
- [ ] 14.2 Run the winit event loop using `EventLoop::run`; on `RedrawRequested` invoke the `pi-oven-render` paint pass
- [ ] 14.3 Implement [crates/pi-oven/src/keys.rs](crates/pi-oven/src/keys.rs) translating `WindowEvent::KeyboardInput` plus current `Modifiers` into a semantic enum (e.g. `KeyAction::CmdDigit(u8)`, `KeyAction::CmdBackquote`, `KeyAction::OptionBackquote`, etc.)
- [ ] 14.4 Log every keyboard event at `debug` showing modifier state and logical key, so the macOS modifier capture can be eyeballed during the prototype
- [ ] 14.5 Add the `dev-crossterm` alternative entry point: under `#[cfg(feature = "dev-crossterm")]`, a parallel `main` that uses `ratatui::backend::CrosstermBackend` and a stdin event loop instead of winit + wgpu — same widget draw path so the experience matches except for keys
- [ ] 14.6 Manual verification: `cargo run -p pi-oven` (default `dev-wgpu`) launches the native app, Cmd+1, Cmd+\`, Option+\`, Cmd+N each appear in `RUST_LOG=debug` output with the correct modifier combination; `cargo run -p pi-oven --no-default-features --features dev-crossterm` launches in a terminal and renders the same `pi-oven` text via the crossterm backend

## 15. Bundle the app

- [ ] 15.1 Install `cargo-bundle` as a developer prerequisite (documented in README; not a runtime dep)
- [ ] 15.2 Run `cargo bundle --release` once and verify `pi-oven.app` is produced with `Info.plist` containing `CFBundleIdentifier`, `CFBundleName`, and `CFBundleShortVersionString`
- [ ] 15.3 Manual verification: double-clicking `pi-oven.app` opens the same window as `cargo run -p pi-oven`

## 16. End-to-end verification

- [ ] 16.1 Run `pnpm --filter pi-oven-server dev`; confirm `"ready"` log line appears, `~/.pi-oven/state.db` exists with the `_migrations` table, log file exists at `~/.pi-oven/logs/`
- [ ] 16.2 Stop the server with Ctrl+C; confirm clean exit and lock file is released (next start succeeds without manual cleanup)
- [ ] 16.3 Drop a sentinel `0002_smoke.sql` (e.g. `CREATE TABLE smoke(x);`) into the migrations dir; restart; confirm a new `state.db.bak.<ts>` was created and the smoke table exists; remove the sentinel afterwards
- [ ] 16.4 Run `cargo run -p pi-oven` (default features); confirm a window opens with `pi-oven` rendered; press Cmd+1 / Cmd+\` / Option+\` and confirm `RUST_LOG=debug` output reflects the modifier+key combinations
- [ ] 16.5 Run `cargo run -p pi-oven --no-default-features --features dev-crossterm`; confirm a terminal UI renders `pi-oven` (modifier-key behaviour not validated here — that's `dev-wgpu`'s job)
- [ ] 16.6 Touch a file in `pi-oven-ui` and rebuild; confirm `cargo build -p pi-oven --timings` output shows only `pi-oven-ui` and `pi-oven` recompiled (validates the crate-split iteration story)
- [ ] 16.7 Run all server tests with `pnpm --filter pi-oven-server test`; confirm green
- [ ] 16.8 Run `cargo test --workspace`; confirm green
