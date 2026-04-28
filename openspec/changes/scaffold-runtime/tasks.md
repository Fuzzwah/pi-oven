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

## 10. Client crate scaffolding

- [ ] 10.1 Create `crates/pi-oven/Cargo.toml` with deps: `winit`, `wgpu`, `glyphon`, `ratatui` (with default features off), `tokio` (rt + macros), `serde` + `serde_json`, `clap`, `tracing`, `tracing-subscriber`, `anyhow`
- [ ] 10.2 Add `[package.metadata.bundle]` (or `Bundle.toml` consumed by `cargo-bundle`) declaring identifier `dev.fuzzwah.pi-oven`, name `pi-oven`, category `public.app-category.developer-tools`, and a placeholder icon path
- [ ] 10.3 Add `tracing_subscriber::fmt::init()` in `main.rs` so `RUST_LOG=info cargo run -p pi-oven` shows logs

## 11. Cell grid and ratatui backend

- [ ] 11.1 Implement [crates/pi-oven/src/render/grid.rs](crates/pi-oven/src/render/grid.rs) defining `Cell { ch: char, fg: Color, bg: Color, attrs: Attrs }` and `Grid { cols, rows, cells: Vec<Cell> }` with `resize(cols, rows)` and `set(x, y, cell)`
- [ ] 11.2 Implement [crates/pi-oven/src/render/backend.rs](crates/pi-oven/src/render/backend.rs) as `RatatuiGridBackend` implementing `ratatui::backend::Backend` by writing the diff into the `Grid`
- [ ] 11.3 Wire a draw step that lays out a `Paragraph::new("pi-oven")` at `Rect::new(0, 0, cols, 1)` and writes it into the grid as the first frame's content
- [ ] 11.4 Unit test the backend: a small `Terminal::draw` call yields the expected cells in the grid

## 12. wgpu + glyphon paint pipeline

- [ ] 12.1 Implement [crates/pi-oven/src/render/paint.rs](crates/pi-oven/src/render/paint.rs) constructing a wgpu `Instance`, `Surface`, `Adapter`, `Device`, `Queue` for the winit window
- [ ] 12.2 Set up a `glyphon::FontSystem`, `SwashCache`, and `TextAtlas`; load a bundled monospace font (e.g. JetBrains Mono Nerd Font subset)
- [ ] 12.3 Build a `glyphon::Buffer` per frame from the cell grid: one styled run per contiguous same-style cell range
- [ ] 12.4 Render the buffer to the surface; clear to a configured background colour first
- [ ] 12.5 Handle `WindowEvent::Resized` and `ScaleFactorChanged`: reconfigure the surface and recompute grid dimensions from window size and font metrics

## 13. Native window and key handling

- [ ] 13.1 Implement [crates/pi-oven/src/main.rs](crates/pi-oven/src/main.rs) opening a winit window with title `pi-oven`, default size (e.g. 1280x800)
- [ ] 13.2 Run the winit event loop using `EventLoop::run`; on `RedrawRequested` invoke the paint pass
- [ ] 13.3 Implement [crates/pi-oven/src/keys.rs](crates/pi-oven/src/keys.rs) translating `WindowEvent::KeyboardInput` plus current `Modifiers` into a semantic enum (e.g. `KeyAction::CmdDigit(u8)`, `KeyAction::CmdBackquote`, `KeyAction::OptionBackquote`, etc.)
- [ ] 13.4 Log every keyboard event at `debug` showing modifier state and logical key, so the macOS modifier capture can be eyeballed during the prototype
- [ ] 13.5 Manual verification: launch the app, press Cmd+1, Cmd+\`, Option+\`, Cmd+N — each appears in the `RUST_LOG=debug` output with the correct modifier combination

## 14. Bundle the app

- [ ] 14.1 Install `cargo-bundle` as a developer prerequisite (documented in README; not a runtime dep)
- [ ] 14.2 Run `cargo bundle --release` once and verify `pi-oven.app` is produced with `Info.plist` containing `CFBundleIdentifier`, `CFBundleName`, and `CFBundleShortVersionString`
- [ ] 14.3 Manual verification: double-clicking `pi-oven.app` opens the same window as `cargo run -p pi-oven`

## 15. End-to-end verification

- [ ] 15.1 Run `pnpm --filter pi-oven-server dev`; confirm `"ready"` log line appears, `~/.pi-oven/state.db` exists with the `_migrations` table, log file exists at `~/.pi-oven/logs/`
- [ ] 15.2 Stop the server with Ctrl+C; confirm clean exit and lock file is released (next start succeeds without manual cleanup)
- [ ] 15.3 Drop a sentinel `0002_smoke.sql` (e.g. `CREATE TABLE smoke(x);`) into the migrations dir; restart; confirm a new `state.db.bak.<ts>` was created and the smoke table exists; remove the sentinel afterwards
- [ ] 15.4 Run `cargo run -p pi-oven`; confirm a window opens with `pi-oven` rendered; press Cmd+1 / Cmd+\` / Option+\` and confirm `RUST_LOG=debug` output reflects the modifier+key combinations
- [ ] 15.5 Run all server tests with `pnpm --filter pi-oven-server test`; confirm green
