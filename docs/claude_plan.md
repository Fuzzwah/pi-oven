# pi-oven — Direct Client/Server TUI for pi-coding-agent

## Context

You currently drive multiple `pi` coding-agent instances across git worktrees through a Rust TUI that runs on a remote host and is accessed over SSH. Operating it through SSH means many useful TUI hotkeys (cmd+`, cmd+1..9, cmd+n, etc.) are intercepted by the local terminal app before reaching the TUI.

`pi-oven` keeps the same UX (sidebar of projects, tabs of active workspaces, conversation pane, input bar — see attached screenshot) but splits it into a native Rust TUI **client** that runs on the Mac and a Node/TS **server** that runs on the LAN/VPN host where pi already runs. The wire is a single WebSocket. Hotkeys land in a native app on macOS, not in a remote terminal, so nothing is stolen by the terminal emulator.

The workflow stays tightly coupled to the way you actually work: every new workspace branches off an up-to-date default branch, and the new-workspace flow walks you through selecting a trigger for the session — Issue, Spec, Skill, or Exploration — before the agent starts, so context is primed correctly the first time. After the work is done, the same agent handles commit, push, and PR creation as tool calls; a paired reviewer agent reads the diff and posts findings; merge is the only deliberately manual step. Projects can optionally define a release branch with its own promotion flow.

---

## High-level architecture

```
┌────────────────────┐          WebSocket+JSON           ┌──────────────────────────┐
│  pi-oven (Rust)    │  ◄──────────────────────────────► │  pi-oven-server (Node)   │
│  ratatui TUI       │  shared-key handshake, optional TLS│  per-workspace pi SDK     │
│  on Mac            │                                    │  (createAgentSession)     │
└────────────────────┘                                    │  + git worktree manager  │
                                                          │  + tracker adapters      │
                                                          │  + OpenSpec scanner      │
                                                          │  + SQLite state          │
                                                          └──────────────────────────┘
                                                                │
                                                                ├── ~/.pi-oven/state.db
                                                                ├── ~/.pi/agent/sessions/  (pi-owned)
                                                                └── user-chosen worktree dirs
```

- One server process, many concurrent workspaces; each workspace owns one in-process pi session via `createAgentSession`.
- Agents are independent of client connection state — server buffers events; client gets a snapshot + replay on reconnect.
- Single user, single shared key. Server binds to a configurable address; TLS optional (LAN OK in plaintext, exposed deployments configure a cert).

---

## Tech stack

**Client** (`crates/pi-oven`) — **native macOS app** wrapping a TUI-style renderer
- Rust 2024
- `winit` for window/event loop and **first-class cmd/option key capture** (the entire reason this project exists; a normal terminal app can't reliably see those modifiers on macOS)
- `wgpu` + `glyphon` for GPU-accelerated monospace text rendering of the cell grid
- `ratatui` for layout and widgets via a **custom `Backend` impl** that writes cells into our grid buffer instead of a terminal
- `tokio` runtime, `tokio-tungstenite` for WebSocket
- `serde` / `serde_json` for wire messages
- `clap` for CLI args (`--server`, `--key`, `--insecure`)
- Config at `~/.config/pi-oven/config.toml` (server URL, key, theme, font)
- Packaged as a `.app` bundle via `cargo-bundle`; menu bar / dock icon owned by us
- Not a terminal app — it draws its own window, so `cmd+1..9`, `cmd+\``, `opt+\``, `cmd+n`, `cmd+w` all hit our event loop directly without the host terminal stealing them

**Server** (`packages/pi-oven-server`, pnpm workspace)
- Node 20+, TypeScript
- `@mariozechner/pi-coding-agent` imported via SDK (`createAgentSession`)
- `ws` for WebSocket
- `better-sqlite3` for state
- `simple-git` for worktree ops
- `octokit` (GitHub) and a thin REST client for Forgejo (Gitea-API-compatible at `/api/v1/`)

---

## Repository layout

```
pi-oven/
├── Cargo.toml                       # workspace
├── crates/
│   └── pi-oven/                     # Native macOS app + TUI renderer
│       ├── src/
│       │   ├── main.rs              # winit event loop entry
│       │   ├── app.rs               # top-level App state, async runtime bridge
│       │   ├── render/              # cell grid + GPU text rendering
│       │   │   ├── mod.rs
│       │   │   ├── backend.rs       # custom ratatui Backend → cell grid
│       │   │   ├── grid.rs          # cell buffer (char, fg, bg, attrs)
│       │   │   └── paint.rs         # wgpu + glyphon paint pass
│       │   ├── ui/                  # ratatui widgets/layout
│       │   │   ├── mod.rs
│       │   │   ├── sidebar.rs       # projects + new-workspace
│       │   │   ├── tabs.rs          # active workspace tabs
│       │   │   ├── conversation.rs  # streamed pi events
│       │   │   ├── input.rs         # message input bar
│       │   │   └── pickers.rs       # issue / spec / skill pickers
│       │   ├── keys.rs              # winit ModifiersState → semantic actions (cmd+1..9, cmd+`, cmd+n, opt+`, etc.)
│       │   ├── net/                 # WebSocket client
│       │   │   ├── mod.rs
│       │   │   ├── codec.rs         # Msg enum (serde-tagged)
│       │   │   └── reconnect.rs     # backoff + replay request
│       │   └── config.rs
│       └── Cargo.toml
├── packages/
│   └── pi-oven-server/              # Node/TS server
│       ├── package.json
│       ├── tsconfig.json
│       └── src/
│           ├── index.ts             # entry, arg parsing
│           ├── server.ts            # WebSocket server + auth
│           ├── protocol.ts          # shared Msg types (mirrors codec.rs)
│           ├── state/
│           │   ├── db.ts            # SQLite schema + migrations
│           │   └── repo.ts          # CRUD: projects, workspaces
│           ├── workspaces/
│           │   ├── manager.ts       # lifecycle, event buffering, replay
│           │   └── session.ts       # one pi SDK session per workspace
│           ├── git/
│           │   ├── worktree.ts      # create/list/remove worktrees
│           │   └── default-branch.ts# fetch + ff-only sync logic
│           ├── trackers/
│           │   ├── index.ts         # adapter interface
│           │   ├── forgejo.ts
│           │   └── github.ts
│           ├── openspec/
│           │   └── scanner.ts       # walk openspec/changes/*/tasks.md
│           ├── skills/
│           │   └── pi-skills.ts     # query pi's skills via SDK
│           └── state/
│               ├── db.ts            # opens DB, runs migrate() on startup
│               ├── migrate.ts       # ~80-line forward-only runner
│               └── migrations/
│                   ├── 0001_initial.sql
│                   └── ...          # 0002_*.sql, 0003_*.ts, etc.
├── package.json                     # pnpm workspace root
├── pnpm-workspace.yaml
├── README.md
└── openspec/                        # already present from your setup-openspec branch
```

---

## SQLite schema and migrations (server)

The schema **will** evolve as we build. Treat every change as a migration from the day we ship the first one.

### Initial schema — [0001_initial.sql](packages/pi-oven-server/src/state/migrations/0001_initial.sql)

```sql
CREATE TABLE _migrations (
  id          INTEGER PRIMARY KEY,        -- numeric prefix from filename
  name        TEXT NOT NULL UNIQUE,       -- full filename
  checksum    TEXT NOT NULL,              -- sha256 of file bytes when applied
  applied_at  INTEGER NOT NULL            -- unix ms
);

CREATE TABLE projects (
  id             INTEGER PRIMARY KEY,
  name           TEXT NOT NULL,
  source_path    TEXT NOT NULL,
  worktree_base  TEXT NOT NULL,
  default_branch TEXT,
  tracker_kind   TEXT,                    -- 'forgejo' | 'github' | NULL
  tracker_base   TEXT,
  tracker_repo   TEXT,
  tracker_token  TEXT,                    -- plaintext v1; encrypted later
  created_at     INTEGER NOT NULL
);

CREATE TABLE workspaces (
  id                  INTEGER PRIMARY KEY,
  project_id          INTEGER NOT NULL REFERENCES projects(id),
  branch              TEXT NOT NULL,
  worktree_path       TEXT NOT NULL,
  origin_kind         TEXT NOT NULL,      -- 'issue' | 'spec' | 'skill' | 'exploration' | 'review'
  origin_ref          TEXT,
  parent_workspace_id INTEGER REFERENCES workspaces(id),  -- set for 'review' workspaces
  tab_order           INTEGER NOT NULL,
  status              TEXT NOT NULL,      -- 'running' | 'idle' | 'closed'
  created_at          INTEGER NOT NULL
);

-- Future migration adds release-branch fields to projects:
--   release_branch          TEXT,        -- null = no release branch
--   release_mode            TEXT,        -- 'manual' | 'auto-on-checks' | null
--   release_required_checks TEXT         -- JSON array of check names

CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

pi sessions themselves stay in pi's own `~/.pi/agent/sessions/`. We only store the path/id.

### Migration model

- **Hand-rolled, forward-only.** No `down` migrations. Every fix is a new forward migration; rollback = stop server + restore backup.
- **Numbered files** in [packages/pi-oven-server/src/state/migrations/](packages/pi-oven-server/src/state/migrations/), zero-padded prefix:
  - `0001_initial.sql` … `0042_rename_workspace_branch.sql`
  - `.sql` for DDL/static seeds; `.ts` for data transforms (`export function up(db: Database): void`)
- **Sorted lexicographically by filename** — the prefix is the order, the suffix is human-readable intent.
- **Applied atomically.** Each migration runs in a single `BEGIN IMMEDIATE` … `COMMIT`. SQLite supports DDL inside transactions, so a failed CREATE/ALTER rolls back cleanly.
- **Tracked with checksums.** On startup we sha256 each file; `_migrations.checksum` must match for already-applied migrations. Mismatch → refuse to start (someone edited a committed migration; that's a bug, not a flaky deploy).

### Migration runner — [state/migrate.ts](packages/pi-oven-server/src/state/migrate.ts)

```
migrate(db, dir):
  CREATE TABLE IF NOT EXISTS _migrations (...)
  applied = SELECT id, name, checksum FROM _migrations
  files   = readdirSync(dir).filter(*.sql|*.ts).sort()
  for each applied row:
    file = files.find(name)
    if !file: ERROR "applied migration <name> missing on disk"
    if sha256(file) !== row.checksum: ERROR "migration <name> tampered"
  pending = files where id not in applied
  if pending.length === 0: return
  log.info("applying N migrations", pending.map(f => f.name))
  backupDb(db.path) -> db.path + ".bak." + now      # before mutating anything
  for each pending file (in order):
    BEGIN IMMEDIATE
    if .sql: db.exec(fileContents)
    if .ts:  await import(file).up(db)
    INSERT INTO _migrations (id, name, checksum, applied_at) VALUES (...)
    COMMIT
  log.info("migrations applied", pending.length)
```

Server **does not start the WebSocket listener** until `migrate()` returns successfully. A half-migrated DB is never visible to clients.

### Backups

- `db.path + ".bak.<unix-ms>"` written via `better-sqlite3`'s `db.backup()` API (online, atomic) immediately before pending migrations run.
- Retain the most recent 10 backups; prune older ones in the same step.
- Manual restore is documented: stop server, copy `.bak.<ts>` over `state.db`, downgrade server binary, start.

### Conventions

- **Never edit a migration after it lands on `main`.** Add a new one. Checksum guard enforces this.
- **One concern per file.** A rename + a new column = two migrations. Easier to bisect later.
- **Filename intent.** `0007_add_workspace_status_idx.sql` beats `0007_misc.sql`.
- **Data + schema split if both needed.** Schema in `.sql`, then a follow-up `.ts` for the data transform. Keeps DDL readable.
- **Tests** in [packages/pi-oven-server/src/state/migrate.test.ts](packages/pi-oven-server/src/state/migrate.test.ts):
  - Empty DB → all migrations apply, expected tables exist.
  - Already-current DB → no-op, no backup taken.
  - Partial DB (only first N applied) → only the rest run.
  - Tampered checksum → runner refuses to start, db unchanged.
  - Pending migration that throws → transaction rolls back, `_migrations` unchanged, backup remains.

### Dev scripts (in [packages/pi-oven-server/package.json](packages/pi-oven-server/package.json))

- `pnpm migrate:status` — print applied + pending lists
- `pnpm migrate:new <slug>` — scaffold next-numbered file (`NNNN_<slug>.sql`)
- `pnpm migrate:reset` — DEV ONLY; deletes `state.db` after typed confirmation, then re-runs all migrations against an empty DB

---

## Wire protocol (WebSocket, JSON)

Single tagged-union `Msg` type, mirrored in `protocol.ts` and `codec.rs`. Selected messages:

**Handshake**
- `C→S Hello { key, client_version }` (first frame)
- `S→C Welcome { server_version, projects: Project[], active_workspaces: WorkspaceSnapshot[] }`
- `S→C AuthFailed { reason }` then close

**Project management**
- `C→S AddProject { kind: 'local-dir'|'local-repo'|'clone', source, target?, name, worktree_base, tracker?: TrackerCfg }`
- `S→C ProjectAdded { project }` / `ProjectError { msg }`
- `C→S RemoveProject { project_id }`

**New-workspace flow**
- `C→S StartNewWorkspace { project_id }` → `S→C IssueList { issues, filters_available }` (if tracker configured) else go straight to spec list
- `C→S PickIssue { issue_id } | SkipIssues`
- `S→C SpecList { specs: { id, name, incomplete_tasks }[] }`
- `C→S PickSpec { spec_id } | SkipSpecs`
- `S→C SkillList { skills }`
- `C→S PickSkill { skill_id } | SkipSkills`
- `C→S ConfirmCreate { project_id, origin: {...}, branch_name }` (server proposes; client may edit)
- `S→C BranchSyncWarning { kind: 'no_remote' | 'fetch_failed' | 'non_ff', detail }` (non-blocking)
- `S→C WorkspaceCreated { workspace }`

**Active session**
- `C→S Send { workspace_id, text, queue_mode: 'steer'|'followup' }`  (mirrors pi's Enter / Alt+Enter)
- `C→S Abort { workspace_id }`  (mirrors Escape)
- `S→C AgentEvent { workspace_id, seq, event }`  (raw pi JSON event passthrough)
- `S→C AgentStatus { workspace_id, status }`

**Reconnect / replay**
- `C→S Resume { workspace_id, last_seq }` → `S→C ReplayBatch { events, latest_seq }`

**Tab / lifecycle**
- `C→S CloseWorkspace { workspace_id, hard?: bool }`
- `C→S ReorderTabs { order: workspace_id[] }`

Each `AgentEvent` carries a monotonic `seq` per workspace — that's the basis of the replay-on-reconnect contract.

---

## Workflow walkthroughs

### Adding a project
1. User picks `+ New project` (cmd+shift+n) → client sends `AddProject`.
2. Server validates: existing dir / `git clone` into target / open existing repo. Detects default branch (`git symbolic-ref refs/remotes/origin/HEAD`, fall back to `git config init.defaultBranch`, fall back to `main`).
3. Persists project; replies with `ProjectAdded`.

### Creating a new workspace
1. Client sends `StartNewWorkspace { project_id }`.
2. Server runs `default-branch.ts::syncDefault(project)`:
   - `git fetch` (skip if no remote)
   - `git checkout <default> && git pull --ff-only`
   - On any failure, capture `{ kind, detail }` and **proceed with local default** (per your "warn and proceed" choice). Warning is sent later attached to `WorkspaceCreated`.
3. If `tracker_kind` set → fetch open issues via adapter → `IssueList`. Else → skip to specs.
4. On `PickIssue`: server proposes branch `issue-<num>-<slug>` (slug ≤ 40 chars, kebab-case from issue title); seeds initial agent context with the issue body + comments.
5. If `SkipIssues`: server scans `openspec/changes/*/tasks.md`, counts unchecked checkboxes (`- [ ]`), returns `SpecList` sorted by user-chosen order (default: most incomplete first).
6. On `PickSpec`: branch `spec-<change-id>`; context seeded with `proposal.md` + `tasks.md`.
7. If `SkipSpecs`: server queries pi SDK for available skills/slash-commands → `SkillList`.
8. On `PickSkill`: branch `skill-<name>-<timestamp>`; context seeded with the skill's prompt.
9. If all skipped (Exploration): branch `explore-<timestamp>`; `origin_kind = 'exploration'`; agent starts with no priming context.
10. Server: `git worktree add <worktree_base>/<branch> -b <branch> <default>`, calls `createAgentSession({ cwd: worktree_path, ...seedContext })`, persists workspace, broadcasts `WorkspaceCreated`.
11. If first user message arrives in an Exploration workspace, server prepends agent system text: "Use /opsx:propose to start a feature, or log an issue if you've found a bug."

### Sending and streaming
- Client `Send { queue_mode }` → server calls `session.queue(text, mode)` (the SDK's Enter / Alt+Enter equivalents).
- pi SDK emits events → server buffers per workspace (ring of last N events + a complete-since-creation log on disk for hard replay) → fans out as `AgentEvent` to connected client.

### Disconnect / reconnect
- Server keeps sessions running, `WorkspaceManager` keeps appending events.
- On reconnect, client sends `Resume { workspace_id, last_seq }` per active workspace; server returns `ReplayBatch` with everything `> last_seq`.

### Post-work flow

The "after the work is done" half of the loop is **agent-driven**: the same `pi` session that did the implementation also commits, pushes, and opens the PR — they're just more tool calls. Code review is delegated to a separate **reviewer agent** spawned in its own paired workspace. Merge triggers automatic worktree cleanup. Optional release branches add a second promotion stage.

**1. Commit to the worktree branch.** The agent runs `git add` / `git commit` in its worktree as it works. pi-oven doesn't micromanage commit cadence — it's a normal `git` tool call against the worktree's branch. The default branch in `source_path` is read-only from the agent's perspective.

**2. Push to remote.** When the user steers "ship it" (or equivalent), the agent runs `git push -u origin <branch>`. The GIT_ASKPASS shim from gotcha 5 supplies the project's `tracker_token` for HTTPS remotes; SSH remotes use the server user's ssh-agent. `GIT_TERMINAL_PROMPT=0` is always set.

**3. Create the pull request.** The agent calls the tracker via `gh` (GitHub) / `tea` (Forgejo) or directly through the tracker adapter. PR title and body are built from the seed context (issue body, spec name, skill prompt) plus a short summary of the diff. Target = the project's default branch.

**4. Spawn the reviewer agent.** The server detects the new PR (tracker webhook with polling fallback) and auto-creates a paired **review workspace**:
- `origin_kind = 'review'`, `origin_ref = <pr_number>`, `parent_workspace_id = <implementation workspace id>`.
- A separate worktree is cut on the PR's head commit (so the original implementation worktree isn't disturbed).
- Appears as its own tab labelled `review #<n>` so you can watch it work or steer it.
- System-level seed: "Review the diff between `<base>` and `<head>` for [issue/spec/skill context]. Post findings as PR review comments via the tracker. Approve only if there are no blocking issues."
- Has tracker write scope (same `tracker_token`); does **not** push to the branch. Comments only.
- When the reviewer agent signals it's done, pi-oven closes the review workspace and removes its worktree.

**5. Code review iteration.** Reviewer agent's comments land on the PR via the tracker API. You read them in the tracker's UI alongside the diff. Address feedback by switching back to the implementation workspace and asking the agent to fix things → it commits and pushes → you (or the system, configurable) trigger a fresh reviewer agent on the updated head.

**6. Merge.** Merging is **always manual** — done in the tracker's UI, not by an agent. When the server sees the PR merged (webhook/poll):
- Implementation workspace: `status = 'closed'`, `git worktree remove --force` + `rm -rf <worktree_path>`, remote branch deleted (the tracker may already do this; we explicitly request it as a safety net), local branch retained by default.
- Review workspace: same cleanup if it's still open.
- Both tabs close in the TUI; if either was the focused tab, focus moves to the next workspace.

**7. Optional: promotion to release branch.** Per-project setting:
- **`release_branch = NULL`** (default): default branch is the only mainline. Step 6 ends the loop.
- **`release_mode = 'manual'`**: TUI exposes a "New release PR" affordance per project. Invoking it opens a `default → <release_branch>` PR; the same review flow (step 4 onward) applies. The PR's required checks come from `release_required_checks`.
- **`release_mode = 'auto-on-checks'`**: server watches tracker check results on `default`; when all `release_required_checks` succeed on a commit, it auto-opens (and optionally auto-merges) the release PR. Useful for projects where promotion is purely gating, not deliberation.

**Why agent-driven push/PR?** It keeps the conversational loop tight — "this looks good, ship it" is the same UX as any other instruction, no context switching. Mitigations for the elevated trust:
- Tokens are scoped per-project; one project's compromise doesn't escalate to others.
- Agents push only to their worktree's branch, never to `default` or `release_branch`.
- The reviewer agent has comment-only scope on the tracker; it cannot push or merge.
- Merge is the only step that crosses into shared state, and it's deliberately manual.

---

## Branch sync details ([git/default-branch.ts](packages/pi-oven-server/src/git/default-branch.ts))

```
syncDefault(project):
  if project.default_branch is null: detect + cache
  in project.source_path:
    if remote 'origin' exists:
      try: git fetch origin
      catch: return { warn: 'fetch_failed', detail }
      try: git checkout <default> && git pull --ff-only origin <default>
      catch: return { warn: 'non_ff', detail }   # uncommitted local commits
    else:
      return { warn: 'no_remote' }
  return { warn: null }
```

Worktree creation always uses local `<default>` after this; warnings are surfaced to the user in the conversation pane as a system-style note attached to workspace creation, not as a blocker.

---

## Tracker adapter interface ([trackers/index.ts](packages/pi-oven-server/src/trackers/index.ts))

```ts
export interface Tracker {
  // Issues — used by the new-workspace picker
  listOpenIssues(opts: { assignee?: string; labels?: string[]; q?: string }): Promise<Issue[]>;
  getIssue(id: number): Promise<IssueDetail>;

  // Pull requests — used by the post-work flow and the reviewer agent
  createPullRequest(opts: { branch: string; base: string; title: string; body: string }): Promise<PullRequest>;
  getPullRequest(number: number): Promise<PullRequestDetail>;        // for state polling
  listPullRequestChecks(number: number): Promise<CheckResult[]>;     // for release auto-promotion
  addReviewComment(number: number, opts: { body: string; path?: string; line?: number }): Promise<void>;
  submitReview(number: number, opts: { state: 'approve' | 'request-changes' | 'comment'; body: string }): Promise<void>;
  deleteRemoteBranch(branch: string): Promise<void>;                  // safety-net cleanup after merge

  // Events — server uses these to detect merges and check completions
  subscribeWebhook?(handler: WebhookHandler): Promise<Unsubscribe>;   // optional; falls back to polling
  pollEvents(since: number): Promise<TrackerEvent[]>;                 // always implemented
}

export function makeTracker(cfg: TrackerCfg): Tracker;
// Switches on cfg.kind: 'github' -> octokit, 'forgejo' -> fetch against /api/v1/repos/{repo}/{issues,pulls}
```

Both backends expose equivalent issue and PR endpoints. The `Tracker` interface is a single surface for both; per-backend differences (e.g. Forgejo's webhook payload shape) are normalised inside the adapter.

---

## Keybindings (client)

Captured by `keys.rs`; macOS-native so terminal app no longer competes:

| Key | Action |
|---|---|
| `cmd+1` … `cmd+9` | Jump to tab N |
| `cmd+\`` / `cmd+shift+\`` | Cycle next/prev tab |
| `opt+\`` / `opt+shift+\`` | Cycle project in sidebar |
| `cmd+n` | New workspace in selected project |
| `cmd+shift+n` | New project |
| `cmd+w` | Close current workspace |
| `enter` (input bar) | Send / steer (pi's Enter behavior) |
| `alt+enter` (input bar) | Send as follow-up (pi's Alt+Enter) |
| `esc` (during agent run) | Abort current turn |

---

## Architecture gotchas to address in v1

These are the failure modes that would otherwise bite mid-build or right after first deploy. Each one has a "what we do in v1" answer baked into the plan.

### 1. Server-side toolchain for the agent

**Why it bites:** the agent expects `rg`, `jq`, `bat`, `fd`, `ast-grep`, `yq`, `sd`, `direnv`, `gh`, plus a forge CLI (`gh` for GitHub, no first-party Forgejo CLI — `tea` works for Gitea/Forgejo). Missing tools mean silent fallback to slower or wronger approaches.

**v1 plan:**
- [packages/pi-oven-server/scripts/bootstrap.sh](packages/pi-oven-server/scripts/bootstrap.sh) — idempotent install for Linux (`apt`/`pacman`/`dnf` detect) and macOS (`brew`); installs the manifest below.
- [packages/pi-oven-server/scripts/tools.manifest.json](packages/pi-oven-server/scripts/tools.manifest.json) — single source of truth: `{ name, package, min_version, why }[]`.
- Server startup self-check: shells out `--version` for each manifest entry; missing/old entries logged as warnings on boot and surfaced via a `S→C ServerStatus { tool_warnings: [...] }` welcome-frame field so the client can show them.
- Manifest seed: `ripgrep`, `jq`, `bat`, `fd-find`, `ast-grep`, `yq`, `sd`, `direnv`, `gh`, `tea`, `git`, `git-lfs`, `node>=20`, `pnpm`.

### 2. macOS-native client window (the *whole reason* for the project)

**Why it bites:** in a normal terminal app on macOS, `cmd+1`, `cmd+\``, `opt+\``, `cmd+n` are intercepted by the OS / terminal emulator before the program sees them. That's the exact pain you're escaping from.

**v1 plan:**
- Client is a real `.app` built with `winit` + `wgpu` + `glyphon` + a custom `ratatui` `Backend` (see Tech Stack). Cmd/Option are first-class via `winit`'s `Modifiers`.
- Packaged with `cargo-bundle`; ships an `Info.plist` and a Dock icon.
- A first-run "Welcome" overlay shows the keymap so muscle memory builds fast.

### 3. Crash/restart recovery

**Why it bites:** server restarts (deploy, OOM, reboot) while workspaces are mid-turn. Without a policy, "running" rows are lies after the next boot.

**v1 plan: eager re-attach.**
- On startup, after migrations, [workspaces/manager.ts](packages/pi-oven-server/src/workspaces/manager.ts) walks every `status='running'` row and calls `createAgentSession({ resume: <pi_session_id>, cwd: <worktree_path> })`.
- Re-attach failures (corrupt session, pi version skew, missing worktree) → mark `idle`, write a one-line reason into the workspace's event log as a synthetic `AgentEvent`, log a structured warning. Workspace stays in the sidebar; opening it shows the reason.
- In-flight tool calls executing in subprocesses are lost across restart — pi will surface this on resume; we don't try to magic them back.
- Verify with the pi SDK: confirm `createAgentSession` accepts a resume option and that mid-turn resume is supported. If only between-turn resume is supported, document that and don't claim mid-turn safety.

### 4. Single-instance lock

**Why it bites:** two `pi-oven-server` processes pointed at the same `~/.pi-oven/` will corrupt SQLite (yes, even WAL) and race on worktrees.

**v1 plan:**
- On startup, `flock(2)` on `~/.pi-oven/server.lock` (exclusive, non-blocking). If held, exit with a clear message naming the holding PID.
- PID + start time written into the lock file body for diagnostics.

### 5. Git authentication in the server's environment

**Why it bites:** the server clones private repos, fetches default branch, may push branches. `git` will hang on a TTY prompt or fail with no creds.

**v1 plan: detect remote scheme, route accordingly.**
- `https://` remotes → `GIT_ASKPASS=<our shim>` invoking `git`. Shim is [packages/pi-oven-server/scripts/askpass.sh](packages/pi-oven-server/scripts/askpass.sh); server passes the project's `tracker_token` via env var per invocation. Token never lands on disk for git.
- `git@`/`ssh://` remotes → use the server user's SSH agent or `~/.ssh/`. Setup documented in README; not pi-oven's job to install keys.
- `GIT_TERMINAL_PROMPT=0` always set so git never blocks waiting for a tty.
- Per-invocation `core.askPass` config to avoid leaking the env var into child processes.

### 6. Per-workspace event log durability

**Why it bites:** `seq`-based replay is only correct if the log survives crashes and `seq` is monotonic and durable.

**v1 plan: append-only NDJSON, rotated by size.**
- `~/.pi-oven/events/<workspace_id>/<created_at>-<rot>.ndjson`, one event per line: `{"seq":N, "ts":<ms>, "event":<pi event>}`.
- Rotate at 64MB; rotation increments `<rot>`. Most workspaces will fit in one file.
- `seq` is server-assigned at the moment of write, persisted **before** fan-out to clients (so clients can never see a `seq` we'd fail to replay).
- Replay = scan files in order, skip lines with `seq <= last_seq`, stream the rest.
- On workspace close → keep the log for N days (configurable, default 14), then gzip. Provides a free "agent transcript history" feature.

### 7. SQLite pragmas at open time

**Why it bites:** "database is locked" under any concurrent reader is a debugging nightmare; FK violations silently pass without `foreign_keys=ON`.

**v1 plan:** [state/db.ts](packages/pi-oven-server/src/state/db.ts) sets, on every open, in this order:
```
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA temp_store = MEMORY;
```

### 8. Worktree orphan cleanup

**Why it bites:** server crashes during `git worktree add`, or someone `rm -rf`s a worktree dir behind our back. `workspaces` rows go out of sync with the filesystem.

**v1 plan:** at the end of [workspaces/manager.ts](packages/pi-oven-server/src/workspaces/manager.ts) startup, after eager re-attach:
- For each `workspaces` row: stat the `worktree_path`. Missing → mark workspace `closed`, log warning.
- For each project: `git worktree prune` to clean up half-created entries.
- Detect worktrees on disk that have no `workspaces` row (e.g. created manually) and log them; do not auto-delete.

### 9. Branch name collisions

**Why it bites:** picking issue #42 twice, or two issues with similar slugs, would collide on branch and worktree path.

**v1 plan:**
- Slug rule: lowercase, kebab, `[a-z0-9-]`, ≤ 40 chars, leading/trailing dashes stripped, empty slug falls back to `change`.
- Existence check pre-creation: if `<branch>` exists OR `<worktree_base>/<branch>` exists, suffix `-2`, `-3` … until free.
- Final branch name proposed back to the client in the `ConfirmCreate` flow so the user can edit before commit.

### 10. WebSocket framing, heartbeat, and payload caps

**Why it bites:** `ws`'s default `maxPayload` is 100MB but pi events with big tool outputs (long `cat` of a file, `rg` over a big tree) can flirt with that; under-the-radar truncation is silent. Macbook sleep leaves half-open sockets that look fine for minutes.

**v1 plan:**
- Server sets `maxPayload: 16 * 1024 * 1024` and rejects oversized frames with a structured `ErrorEvent` (rather than disconnecting).
- For pi events that exceed the cap, server splits them into a synthetic `LargeEventChunk { id, seq, idx, total, payload }` sequence; client reassembles. Most events won't trigger this; we just need it to not corrupt anything when it does.
- Ping every 30s, dead-client timeout at 90s. Server closes the socket; client's reconnect loop kicks in.
- Client reconnect loop: exponential backoff with jitter, capped at 30s, infinite retries while the app is open.

### 11. Pi session lifecycle quirks

**Why it bites:** pi owns its own session files. We make assumptions about session ids, resumption, and what happens when pi crashes.

**v1 plan:**
- A spike task in slice 1: read pi's SDK source under `packages/coding-agent` to confirm: (a) session-resume API surface, (b) what an SDK-level error looks like, (c) whether multiple sessions in one Node process share state. Capture findings in [docs/pi-sdk-notes.md](docs/pi-sdk-notes.md).
- Default behavior: never delete pi's session files; we treat them as authoritative for agent memory. Closing a workspace just closes our handle.
- Pin `@mariozechner/pi-coding-agent` exactly in [packages/pi-oven-server/package.json](packages/pi-oven-server/package.json); upgrade is a deliberate PR.

### 12. Spawn environment for pi and for shell tools it runs

**Why it bites:** missing `LANG`/`LC_ALL` → unicode mojibake; missing `PATH` entries → `tool not found`; uncontrolled `EDITOR` → tools that try to open an editor hang forever; inherited `TERM` → ANSI codes leak into our event stream.

**v1 plan:** [workspaces/session.ts](packages/pi-oven-server/src/workspaces/session.ts) builds an explicit env per pi session:
- `PATH` = manifest tool paths + `/usr/local/bin:/usr/bin:/bin` + per-project mise/asdf shims if detected
- `LANG=en_US.UTF-8`, `LC_ALL=en_US.UTF-8` (or whatever the server config says)
- `TZ` from server config, default `UTC`
- `EDITOR=true` (no-op so tools never block on an interactive editor)
- `TERM=dumb`, `NO_COLOR=1` (we render the events; we don't want ANSI noise inside payloads)
- `GIT_TERMINAL_PROMPT=0`
- `PI_OVEN_WORKSPACE_ID=<id>` for traceability

### 13. Tracker reliability

**Why it bites:** rate limits, paginated issue lists, network flakes, expired tokens — all currently fail the new-workspace flow.

**v1 plan:**
- Adapters paginate up to 200 open issues by default; UI exposes a search box that re-queries with the tracker's search.
- 5-minute in-memory ETag cache per project; backed by SQLite for persistence across restarts (table `tracker_cache(project_id, key, etag, body, fetched_at)`).
- Token failure → user-visible error in the picker with "Reconfigure tracker" affordance. Workspace creation isn't blocked: the user can always skip to specs/skills/clean.

### 14. Structured logging from day 1

**Why it bites:** server runs as a service with no terminal; "what just happened" without good logs is a session of grief.

**v1 plan:**
- `pino` for structured JSON logs.
- Levels: `error`, `warn`, `info` default, `debug` opt-in via `PI_OVEN_LOG_LEVEL`.
- Every log line carries `workspace_id` when relevant (via `pino`'s child loggers).
- Logs go to `~/.pi-oven/logs/server-<date>.ndjson`, rotated daily, last 7 days kept.
- `pi-oven-server logs --workspace <id> --tail` admin subcommand for live debugging.

### 15. Type-sharing between `protocol.ts` and `codec.rs`

**Why it bites:** they will drift. A field rename on the server breaks the client silently in production.

**v1 plan: golden fixtures + round-trip tests.**
- [packages/pi-oven-server/test/fixtures/protocol/](packages/pi-oven-server/test/fixtures/protocol/) — one `.json` per message kind, manually authored.
- TS test: every fixture parses, re-serializes, deep-equals the original.
- Rust test: same fixtures, same property, in [crates/pi-oven/tests/protocol_fixtures.rs](crates/pi-oven/tests/protocol_fixtures.rs).
- CI runs both. Adding a new message means adding a fixture in the same PR — easy social contract.
- Defer codegen (`ts-rs`, `typeshare`) until the protocol churn rate justifies it.

### 16. Disk space and worktree lifecycle

**Why it bites:** `node_modules`, `target/`, build artifacts make worktrees fat. Closing a workspace without cleanup → silent disk bloat.

**v1 plan:**
- Closing a workspace (`hard: true`): `git worktree remove --force` then `rm -rf` the path. Branch is **kept** by default — losing the branch loses the work.
- Closing a workspace (`hard: false`, default): just remove from the active tabs list. Worktree stays for resume.
- Periodic (daily) `git gc --auto` per project.
- `pi-oven-server fsck` admin subcommand surfaces orphan worktrees, oversized event logs, missing branches.

### 17. Server config and secrets

**Why it bites:** baking the shared key into a CLI flag means it ends up in shell history and `ps`.

**v1 plan:**
- Config at `~/.pi-oven/server.toml` (file mode 600 enforced on startup). Fields: `bind`, `key_file`, `data_dir`, `tls_cert`, `tls_key`, `log_level`, `tz`.
- `key_file` points to a separate file (so the key can be in a secret manager / agenix / sops mount).
- Env var overrides: `PI_OVEN_BIND`, `PI_OVEN_KEY_FILE`, `PI_OVEN_DATA_DIR`.
- CLI flags only for one-off operational stuff (`--migrate-only`, `--fsck`).

### 18. CSWSH and origin checks

**Why it bites:** even on a LAN, an attacker on the same network can pivot via a malicious page in the user's browser if the WebSocket has no origin policy.

**v1 plan:**
- Server requires an `Origin` header that either matches a configured allowlist or is absent (native client sends no Origin, which we treat as OK; any browser will send one and must match).
- Default allowlist: empty. Browser-based clients are out of scope for v1.

---

## Sequencing (within the full-workflow MVP)

Even though the target is end-to-end, build in slices that each leave a working app. Each slice is shippable on its own.

0. **Foundations (de-risk first)** — pi SDK spike (verify resume API, error semantics, multi-session safety) → notes in [docs/pi-sdk-notes.md](docs/pi-sdk-notes.md); winit + wgpu + glyphon + ratatui-custom-backend prototype that renders "hello world" in a window and reports cmd+1 / opt+\` / cmd+n events; toolchain manifest + bootstrap script. These three are the highest-uncertainty pieces; cheaper to validate before the rest of the architecture commits to them.
1. **Skeleton** — Cargo + pnpm scaffolding, single-instance lock, SQLite + migration runner with `0001_initial.sql`, structured logging, WebSocket handshake with shared key + Origin policy, frame heartbeat, render the same panes (empty), single hard-coded workspace with pi SDK round-trip and event log written to NDJSON.
2. **Multi-workspace + tabs** — `WorkspaceManager`, tabs UI, hotkeys, eager re-attach on startup, replay-on-reconnect against the NDJSON log, worktree orphan cleanup.
3. **New-workspace pickers** — Exploration → skill picker → spec picker → issue picker (in that order; each adds an external dependency: pi SDK skill listing → openspec scanner → tracker adapter).
4. **Add-project flow** — local dir, local repo, clone-from-URL with target-path prompt; tracker config UI per-project; GIT_ASKPASS shim wired up; default-branch sync with warn-and-proceed.
5. **Tracker event observability + merge cleanup** — tracker adapter PR methods (`createPullRequest`, `getPullRequest`, `listPullRequestChecks`, `deleteRemoteBranch`); webhook receiver with polling fallback; PR-merged detection drives auto-cleanup of the implementation workspace (worktree remove, remote branch delete, status closed). Agent-driven commit/push/PR-open flow works end-to-end against this.
6. **Reviewer agent** — paired-workspace model (`origin_kind = 'review'`, `parent_workspace_id`); separate worktree on PR head; system seed + tracker write scope (comments only); review tab in the TUI; cleanup on reviewer-done signal.
7. **Release branch flow** — per-project `release_branch` / `release_mode` / `release_required_checks`; manual "New release PR" affordance; auto-on-checks watcher reading tracker check results; same review/merge path as default-branch PRs.
8. **Polish** — branch-sync warnings surfaced in UI, tool-warnings banner, error toasts, theme parity with the screenshot, config file, `fsck` admin command, release-status indicators in tabs.

This sequencing also lets you start using it for real work after slice 2.

---

## Verification plan

End-to-end smoke test that proves the whole thing hangs together:

1. **Server up**: `pnpm --filter pi-oven-server dev -- --bind 0.0.0.0:7373 --key $(cat ~/.pi-oven/key)` — exits cleanly, SQLite created.
2. **Client connects**: `cargo run -p pi-oven -- --server ws://lan-host:7373 --key-file ~/.pi-oven/key` — handshake succeeds, sidebar shows empty project list.
3. **Add project from clone**: cmd+shift+n, supply a Forgejo URL and a target path; project appears, default branch detected.
4. **Create workspace from issue**: cmd+n on the project, pick an open issue from the list. Verify branch name is `issue-<n>-<slug>`, worktree exists at the configured base, agent receives issue body as initial context.
5. **Disconnect/reconnect**: kill the client mid-stream while pi is responding; relaunch; verify the streamed turn finishes and replay fills the conversation pane (no missing events).
6. **Spec flow**: cmd+n → skip issues → spec list shows correct unchecked-task counts (cross-check by `grep -c '^- \[ \]' openspec/changes/*/tasks.md`).
7. **Skill flow**: cmd+n → skip issues, skip specs → skill list mirrors pi's available skills.
8. **Clean flow**: skip everything → agent's first system nudge mentions `/opsx:propose` or logging an issue.
9. **Branch-sync warning**: simulate a non-ff state by adding an unpushed commit to default; verify a warning attaches to `WorkspaceCreated` and shows in the conversation pane but doesn't block.
10. **Tab cycling**: open three workspaces; cmd+1/2/3 jumps directly; cmd+\` cycles; cmd+w closes one and reorders the rest.

Run unit tests for: `default-branch.ts` (mocked git), tracker adapters (record/replay HTTP fixtures), `openspec/scanner.ts` (sample fixture trees), `protocol.ts` round-trip with `codec.rs` (golden JSON files shared between languages), and `migrate.ts` (see migration test list above).

11. **Migration upgrade smoke**: from a freshly-built `state.db`, drop a sentinel new migration in `migrations/` (e.g. `0002_add_workspace_label.sql` adding a `label TEXT` column with default), restart the server. Verify: a `state.db.bak.<ts>` was created, the new column exists, `_migrations` shows two rows, second startup is a no-op (no new backup).

---

## Open items deliberately deferred

- TLS termination and cert management — start with shared-key over WS; layer WSS on a configurable cert later.
- Token encryption at rest in SQLite — store plaintext for v1, add when this is exposed beyond your own VPN.
- Multi-user / per-user workspaces — out of scope per your single-user choice.
- Mobile or web client — not in scope; this is a Rust TUI for a Mac.
- Pi version pinning / upgrade path — server's `package.json` pins `@mariozechner/pi-coding-agent`; bump deliberately.

---

## Critical files to create

**Client (Rust)**
- [Cargo.toml](Cargo.toml) (workspace root)
- [crates/pi-oven/Cargo.toml](crates/pi-oven/Cargo.toml) — winit, wgpu, glyphon, ratatui, tokio, tokio-tungstenite, serde, clap
- [crates/pi-oven/src/main.rs](crates/pi-oven/src/main.rs) — winit event loop entry
- [crates/pi-oven/src/render/backend.rs](crates/pi-oven/src/render/backend.rs) — custom `ratatui::backend::Backend` writing to a cell grid
- [crates/pi-oven/src/render/paint.rs](crates/pi-oven/src/render/paint.rs) — wgpu + glyphon paint pass
- [crates/pi-oven/src/keys.rs](crates/pi-oven/src/keys.rs) — winit modifiers → semantic actions
- [crates/pi-oven/src/net/codec.rs](crates/pi-oven/src/net/codec.rs) — must match `protocol.ts` exactly
- [crates/pi-oven/src/ui/](crates/pi-oven/src/ui/) — sidebar, tabs, conversation, input, pickers
- [crates/pi-oven/tests/protocol_fixtures.rs](crates/pi-oven/tests/protocol_fixtures.rs) — golden-fixture round-trip tests
- [crates/pi-oven/Info.plist](crates/pi-oven/Info.plist) + [crates/pi-oven/Bundle.toml](crates/pi-oven/Bundle.toml) — `cargo-bundle` config

**Server (Node/TS)**
- [packages/pi-oven-server/package.json](packages/pi-oven-server/package.json) — pinned `@mariozechner/pi-coding-agent`, scripts (`dev`, `migrate:*`, `fsck`)
- [packages/pi-oven-server/src/index.ts](packages/pi-oven-server/src/index.ts) — entry, lockfile, config load, migrate, log boot
- [packages/pi-oven-server/src/server.ts](packages/pi-oven-server/src/server.ts) — WebSocket listener, Origin policy, heartbeat, frame caps
- [packages/pi-oven-server/src/protocol.ts](packages/pi-oven-server/src/protocol.ts)
- [packages/pi-oven-server/src/config.ts](packages/pi-oven-server/src/config.ts) — `~/.pi-oven/server.toml` loader + env overrides
- [packages/pi-oven-server/src/log.ts](packages/pi-oven-server/src/log.ts) — pino root + child-logger helpers
- [packages/pi-oven-server/src/workspaces/manager.ts](packages/pi-oven-server/src/workspaces/manager.ts) — lifecycle, eager re-attach, orphan cleanup, NDJSON event log + replay
- [packages/pi-oven-server/src/workspaces/session.ts](packages/pi-oven-server/src/workspaces/session.ts) — pi SDK adapter, explicit spawn env
- [packages/pi-oven-server/src/git/default-branch.ts](packages/pi-oven-server/src/git/default-branch.ts)
- [packages/pi-oven-server/src/git/worktree.ts](packages/pi-oven-server/src/git/worktree.ts)
- [packages/pi-oven-server/src/git/auth.ts](packages/pi-oven-server/src/git/auth.ts) — askpass shim invoker, ssh/https detection
- [packages/pi-oven-server/src/trackers/index.ts](packages/pi-oven-server/src/trackers/index.ts)
- [packages/pi-oven-server/src/trackers/forgejo.ts](packages/pi-oven-server/src/trackers/forgejo.ts)
- [packages/pi-oven-server/src/trackers/github.ts](packages/pi-oven-server/src/trackers/github.ts)
- [packages/pi-oven-server/src/openspec/scanner.ts](packages/pi-oven-server/src/openspec/scanner.ts)
- [packages/pi-oven-server/src/skills/pi-skills.ts](packages/pi-oven-server/src/skills/pi-skills.ts)
- [packages/pi-oven-server/src/state/db.ts](packages/pi-oven-server/src/state/db.ts) — pragmas + migrate() on open
- [packages/pi-oven-server/src/state/migrate.ts](packages/pi-oven-server/src/state/migrate.ts)
- [packages/pi-oven-server/src/state/migrations/0001_initial.sql](packages/pi-oven-server/src/state/migrations/0001_initial.sql)
- [packages/pi-oven-server/src/admin/fsck.ts](packages/pi-oven-server/src/admin/fsck.ts)
- [packages/pi-oven-server/scripts/bootstrap.sh](packages/pi-oven-server/scripts/bootstrap.sh)
- [packages/pi-oven-server/scripts/tools.manifest.json](packages/pi-oven-server/scripts/tools.manifest.json)
- [packages/pi-oven-server/scripts/askpass.sh](packages/pi-oven-server/scripts/askpass.sh)
- [packages/pi-oven-server/test/fixtures/protocol/](packages/pi-oven-server/test/fixtures/protocol/) — golden JSON, shared with the Rust round-trip test

**Docs / project**
- [docs/pi-sdk-notes.md](docs/pi-sdk-notes.md) — output of slice-0 SDK spike (resume API, error semantics, multi-session safety)
- [README.md](README.md) — install, bootstrap, server config, SSH/git auth setup, first-run walkthrough
