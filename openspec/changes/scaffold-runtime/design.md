## Context

pi-oven is a direct client/server replacement for an SSH-tunnelled TUI that wraps the `pi` coding agent across multiple git worktrees. The full v1 plan is captured in [docs/claude_plan.md](../../../docs/claude_plan.md). This first change establishes the runtime bedrock both halves of the system will build on, with no functionality beyond "boot, prove the core pieces work, exit cleanly."

Two architectural commitments fall out of the v1 plan that shape every decision below:

1. The client is a **native macOS app**, not a terminal program. The whole reason this project exists is that running a TUI inside a terminal on macOS means cmd/option keys never reach the program — they're consumed by the OS or terminal emulator first. We render a text grid ourselves, in our own window.
2. The server is the canonical state owner. Schema *will* evolve. Migrations are a first-class concern from day one, not a bolt-on.

## Goals / Non-Goals

**Goals**

- A `cargo run -p pi-oven` opens a native macOS window that captures cmd/option keys.
- A `pnpm --filter pi-oven-server dev` boots the server: config → lock → log → DB+migrate → "ready" log line.
- Migration runner is forward-only, checksum-verified, atomic per-migration, with automatic backup before applying anything pending.
- Two independent processes can never share `~/.pi-oven/` state.
- Layout, scripts, and pragmas are right enough that subsequent changes never need to revisit them.

**Non-Goals**

- No WebSocket listener, no networking, no client/server connection in this change.
- No pi SDK integration. Importing `@mariozechner/pi-coding-agent` and proving session creation comes in a later change.
- No real UI: the client renders one placeholder string, not a sidebar or tabs.
- No auth, no TLS, no tracker adapters, no worktree management.
- No CI / release pipeline. Local dev only.
- No Linux/Windows client builds. Server is portable; client is macOS-only.

## Decisions

### D1. Client uses winit + wgpu + glyphon, not a terminal

**Why:** the entire premise of the project. Terminal apps on macOS cannot reliably observe cmd/option modifiers. A native window owns its key events.

**Alternatives considered:**
- Kitty keyboard protocol in a normal terminal — works in some terminals but not portably; cmd+\` etc. still need terminal-side keymap config, defeating the point.
- Tauri / webview — pulls in a webview, defeats the lightweight-TUI feel, complicates packaging.
- AppKit directly via `objc2` — most native, but considerably more glue code than winit gets us for free; revisit only if winit limitations bite.

**Trade-off:** we own a render pipeline. Mitigated by reusing `ratatui` for layout/widgets via a custom `Backend` impl that writes into our cell grid — we get ratatui's expressive layout DSL without ratatui's terminal IO assumptions.

### D2. Server is Node/TS importing the pi SDK in-process

**Why:** the pi SDK (`createAgentSession`) is published as a Node package; importing it directly avoids an RPC subprocess and gives us the full SDK surface (events, slash commands, skills) in TypeScript types. This change doesn't actually use the SDK yet, but the choice anchors the rest of the v1 plan, so the package skeleton is set up to add it later without re-architecting.

**Alternatives considered:** spawn pi via `--mode rpc` (language-agnostic but loses SDK-level introspection); rewrite a server in Rust talking to pi's RPC mode (uniform language but doubles the surface area for a single-user tool).

### D3. Hand-rolled forward-only migration runner

**Why:** schema evolution is a known-painful area, and we want to *understand* every migration that runs. A ~80-line runner with checksums is auditable; a library locks us into another upgrade treadmill. Forward-only matches single-user reality — the rollback path is "stop server, restore the timestamped backup, downgrade binary," not "run `down` migrations that nobody tests."

**Alternatives considered:** Kysely's migrator (good, but couples query layer); Drizzle auto-diff (powerful, opaque on rename/refactor — we don't want surprises in DDL); umzug (generic but heavier).

**Trade-off:** more code we own. Mitigated by aggressive tests for the runner itself (apply on empty / current / partial / tampered / throwing), so the muscle is exercised early.

### D4. Single-instance lock via flock on `~/.pi-oven/server.lock`

**Why:** SQLite WAL is concurrent-reader safe, not "two whole servers writing through different connections" safe. Lock file enforces single-process ownership of the data directory.

**Decision:** open `~/.pi-oven/server.lock` with `O_RDWR|O_CREAT`, attempt `flock(LOCK_EX|LOCK_NB)`. On EWOULDBLOCK: read the file body (PID + start time written on successful acquire), exit non-zero with a message naming the holder. Lock is released on process exit; the OS handles abandoned locks correctly across SIGKILL.

**Alternative:** PID file alone — fragile across crashes, race-prone. `proper-lockfile` npm package — works but spins instead of using `flock`; we want the kernel to do the work.

### D5. SQLite pragmas applied unconditionally at every open

**Why:** these are settings whose absence causes hours of mystery debugging. Setting them in `db.ts` means there is no other code path that can open the DB without them.

```
PRAGMA journal_mode = WAL;       -- concurrent readers; durable writes
PRAGMA synchronous = NORMAL;     -- WAL-appropriate; trade tiny crash window for throughput
PRAGMA foreign_keys = ON;        -- they're off by default, which is wrong
PRAGMA busy_timeout = 5000;      -- 5s wait before SQLITE_BUSY; covers migration windows
PRAGMA temp_store = MEMORY;      -- avoid spurious tmpfile churn
```

### D6. Migration backup uses `db.backup()`, not file copy

**Why:** `better-sqlite3`'s `db.backup()` uses SQLite's online backup API — atomic, safe even if writers are active (none should be in our boot sequence, but belt-and-braces). File-copy of an active WAL DB can capture an inconsistent snapshot.

**Decision:** before any pending migration runs, call `db.backup(<path>.bak.<unix-ms>)`. Keep the most recent 10 backups; prune older same step.

### D7. Logs are NDJSON via pino, written to disk from the start

**Why:** the server runs as a service with no terminal. Searchable/filterable logs from day 1 makes every later "what just happened" debug session 5 minutes instead of 5 hours.

**Decision:** pino root logger writes to `~/.pi-oven/logs/server-<YYYY-MM-DD>.ndjson`, daily rotation, last 7 days kept. Levels: `error`/`warn`/`info` default, `debug` opt-in via env. Development mode (`NODE_ENV=development`) also pretty-prints to stdout.

### D8. Config is `~/.pi-oven/server.toml`, env vars override

**Why:** secrets (eventually) and environment-specific values (paths, bind address) belong outside the code repo. TOML keeps it human-editable; env overrides keep systemd/agenix workflows easy.

**Schema (this change populates only the fields it actually reads):**
```toml
data_dir = "~/.pi-oven"      # default; overridable
log_level = "info"
tz = "UTC"
# bind, key_file, tls_*, etc. land in the WebSocket change
```

Env overrides: `PI_OVEN_DATA_DIR`, `PI_OVEN_LOG_LEVEL`, `PI_OVEN_TZ`. File mode 0600 enforced on startup; refuse to start if looser.

### D9. Initial migration creates only the `_migrations` table

**Why:** every other table belongs to a feature change and should land alongside the change that uses it. Conflating "boot scaffolding" with "every table we'll ever need" makes both harder to reason about.

**Decision:** `0001_initial.sql` contains exactly:
```sql
CREATE TABLE _migrations (
  id          INTEGER PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  checksum    TEXT NOT NULL,
  applied_at  INTEGER NOT NULL
);
```
Feature changes append `0002_*.sql` etc. as they need them.

### D10. Client packages as a `.app` via cargo-bundle

**Why:** macOS first-class — Dock icon, Info.plist, no terminal window in front. Also: launching a `.app` keeps cmd-Tab and Mission Control sane.

**Decision:** `cargo-bundle` config in [crates/pi-oven/Bundle.toml](crates/pi-oven/Bundle.toml) (or `[package.metadata.bundle]` in `Cargo.toml`); `cargo bundle --release` produces `pi-oven.app`. Dev workflow remains `cargo run -p pi-oven`, which launches an unbundled binary that opens a window directly — fine for development.

## Risks / Trade-offs

- **[Risk] winit's macOS modifier handling is correct but our renderer is unfamiliar territory.** Mitigation: scope this change to a *placeholder render only* — a single string in a window. Validate the render pipeline before scaling it up; defer all real UI to later changes.
- **[Risk] `flock` semantics are different on macOS (BSD flock) vs Linux.** Mitigation: rely on Node's `fs.openSync` + a small wrapper that uses the libc `flock(2)` via `node:fs/promises` is not enough — we'll use `proper-lockfile`'s file-based lock with `realpath` (it falls back to `O_EXCL` semantics that work identically across both). If `proper-lockfile`'s spin behaviour bites later we can swap to a native binding.
- **[Risk] cargo-bundle's macOS support is OK but quirky around code signing.** Mitigation: out of scope for this change. Local unsigned `.app` is fine for v1; signing/notarisation is a future change before any wider distribution.
- **[Trade-off] Forward-only migrations means a bad migration requires backup-restore.** Accepted explicitly per the plan; the per-pending-batch backup is the safety net.
- **[Trade-off] Single-instance lock prevents an "operator runs a quick fsck while server is up" workflow.** Accepted — admin commands will use a separate read-only lock or run a one-shot subcommand under the same lock.

## Migration Plan

This change introduces the project — there's nothing to migrate from. The only operational concern is that **future migrations must follow the conventions established here**:

- Numbered, zero-padded prefix (`0002`, `0003`, …).
- One concern per migration file.
- Filename intent in the slug (`0007_add_workspace_status_idx.sql`, not `0007_misc.sql`).
- Never edit a migration after it has been applied to any non-throwaway DB. Checksum guard enforces this.

Rollback for *this* change = `rm -rf ~/.pi-oven/` and uninstall.

## Open Questions

- **Does winit on macOS surface `Cmd+\`` cleanly through `Modifiers + Key::Backquote`?** Needs validation in the prototype — if not, we may need a small AppKit-level shim. Logged as a slice-0 risk in the plan; this change is the place we find out.
- **`proper-lockfile` vs a tiny native flock binding** — start with `proper-lockfile` for portability; revisit if its retry/staleness semantics produce surprises.
