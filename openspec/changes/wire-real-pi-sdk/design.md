## Context

The previous slice (`live-session`) wired the full conversation pipeline — server `WorkspaceManager`, `AgentSession`, NDJSON event log, ring buffer, WebSocket fan-out, Rust conversation pane, `Resume`/`ReplayBatch` — but exercised it only with a synthetic event stub gated by `PI_OVEN_SDK_STUB=1`. The real SDK branch in [packages/pi-oven-server/src/workspaces/session.ts:50](packages/pi-oven-server/src/workspaces/session.ts) just throws. The user has `~/.pi/agent/auth.json` (Anthropic + GitHub Copilot OAuth) and `~/.pi/agent/settings.json` (`defaultProvider=github-copilot`, `defaultModel=claude-sonnet-4.6`) configured, so the SDK is one `createAgentSession()` call away from working.

The `@mariozechner/pi-coding-agent` SDK's event stream is richer than the protocol the renderer currently understands: `message_update` carries inner `assistantMessageEvent` deltas with `contentIndex`/`partial`, tool calls are split across `tool_execution_start`/`update`/`end` events keyed by `toolCallId`, and there are turn/agent/message/queue/compaction/auto-retry lifecycle events. The renderer ([crates/pi-oven-ui/src/conversation.rs:21](crates/pi-oven-ui/src/conversation.rs)) consumes a flat stream of `text_delta { text }`, `tool_call { tool_name | name, args | input }`, `tool_result { output }`, `status` — established and validated against the stub.

## Goals / Non-Goals

**Goals:**

- Make `pnpm --filter pi-oven-server dev` (no stub flag) drive a real LLM session end-to-end.
- Keep the live-session wire contract and the conversation renderer untouched.
- Fail server boot fast and loudly when auth/model is missing, with an actionable message.
- Keep `PI_OVEN_SDK_STUB=1` working unchanged for tests and offline dev.
- Confine SDK API surface to `session.ts` so future SDK upgrades are a single-file diff.

**Non-Goals:**

- Multi-workspace, picker UI, worktree creation. Still one hardcoded workspace.
- Renderer enrichment for thinking blocks, partial tool results, or other SDK-only event types. Future change.
- Per-tool environment isolation via custom `BashOperations`. Process-level env defaults are best-effort; gotcha 12 is a future slice.
- Resuming the same pi session file across server restarts. A fresh session is created each boot; the user's pi sessions dir grows accordingly until a future cleanup change.
- Multimodal input (image attachments). Plain text only.

## Decisions

### Translate SDK events server-side rather than passing through

**Decision:** A pure function `translateSdkEvent(ev)` in `session.ts` maps `AgentSessionEvent` to either `{ kind: "event", event }` (the simple shape the renderer already understands), `{ kind: "status", status }` (for `agent_start`/`agent_end`), or `{ kind: "drop" }` (everything else).

**Alternatives considered:**

- *Passthrough:* serialize SDK events verbatim and update the Rust renderer to understand the richer shapes. Rejected for v1: blast radius is large (renderer + golden fixtures + `conversation-pane` capability spec all change), and we have no UX story for thinking blocks or partial tool results yet. Translation defers that decision until we have one.
- *Translate inside the renderer:* keep raw SDK events on the wire, do the simplification in the Rust client. Rejected: pushes pi SDK schema knowledge into a Rust crate that has no other coupling to the SDK, and would require duplicating any future translation logic in tests on both sides.

**Why this is reversible:** The translator is a single pure function. A future change can replace it with passthrough by deleting it, switching `subscribe()` to forward `ev` directly, and evolving the renderer — no other code is affected.

### Lazy-init the SDK session, but eagerly from `WorkspaceManager.init()`

**Decision:** `AgentSession` gets a new `async init()` that calls `createAgentSession`. `WorkspaceManager.init()` awaits it before returning. Net effect: the WebSocket listener never binds if SDK init fails.

**Why:** Failing fast at boot beats a late failure on the user's first message — the user gets a clear log line with `step: "workspace_manager"` and a "no usable model" message instead of a confusing "session not ready" surfaced through `ErrorEvent` mid-conversation. Aligns with the existing boot pipeline's structured error reporting in [packages/pi-oven-server/src/index.ts:107](packages/pi-oven-server/src/index.ts).

### Use `AgentSession.prompt(text, { streamingBehavior })` as the unified entry point

**Decision:** The existing `queue(text, mode)` wrapper calls `this.piSession.prompt(text, { streamingBehavior: mode === "steer" ? "steer" : "followUp" })`. One method handles both "first message of the session" and "queue while streaming."

**Alternatives considered:** `sendUserMessage` for first message + `steer`/`followUp` for streaming queue. Rejected as a needless branching condition (`this.piSession.isStreaming`) — `prompt` already encapsulates that logic per the SDK docs at [agent-session.d.ts:307–315](https://example.invalid).

### `cwd = <data_dir>/workspaces/1`, derived not configured

**Decision:** `WorkspaceManager.init` computes `cwd = join(dataDir, "workspaces", "1")` and `mkdir`s it `0700`. No new config field.

**Alternatives considered:**

- `process.cwd()`: couples the workspace to the shell that started the server. Rejected.
- New `[workspace] dir = "..."` TOML field + `PI_OVEN_WORKSPACE_DIR` env var: more flexibility. Deferred until multi-workspace lands and there's a real reason to relocate it.

**Why this dir:** mirrors the existing `<data_dir>/events/<id>/` pattern, survives server restarts, will become the natural home for the workspace 1 worktree when that slice arrives.

### Best-effort process-env application for gotcha 12

**Decision:** A `applyChildProcessEnv()` helper sets `LANG`, `TZ`, `EDITOR=true`, `TERM=dumb`, `NO_COLOR=1`, `GIT_TERMINAL_PROMPT=0`, `PI_OVEN_WORKSPACE_ID=<id>` on `process.env`, **only if the key is currently unset**. Called once from `AgentSession.init()` before `createAgentSession`.

**Why this is "best effort":** the pi SDK runs in-process; its bash tool inherits `process.env`. True per-tool env isolation (the proper gotcha-12 fix) requires constructing a custom `BashOperations` and threading it through `customTools` — out of scope for this slice. The only-if-unset rule prevents clobbering deliberately-set user values like `LANG=de_DE.UTF-8`.

### Keep `dispose()` minimal

**Decision:** `AgentSession.dispose()` unsubscribes and calls `piSession.dispose()`. No SIGINT/SIGTERM handler wiring in this change — the server today doesn't dispose `WorkspaceManager` on shutdown either. We add `dispose()` for completeness so a future shutdown-cleanup change has a hook to call.

## Risks / Trade-offs

- [Risk] **SDK upgrades break the translator silently.** The mapping table relies on event-type strings and field names from `@mariozechner/pi-agent-core` and `@mariozechner/pi-coding-agent`. → **Mitigation:** pin the version exactly (`0.70.6`); the translator unit test asserts every mapped row, so a field rename in a future bump fails the test loudly. Upgrades are an explicit PR.

- [Risk] **In-process bash inherits the server's full `process.env`.** Secrets the user has exported in their shell (e.g. `GITHUB_TOKEN`) become visible to any tool the agent runs. → **Mitigation:** documented as a known limitation in the proposal's Out-of-scope. Future change adds custom `BashOperations` with a scrubbed env. Until then, the threat model is "the agent runs locally as the user already does."

- [Risk] **`createAgentSession` creates a new pi session file every server boot.** Over weeks the user's `~/.pi/agent/sessions/` accumulates. → **Mitigation:** trivial; user can `rm -rf ~/.pi/agent/sessions/*` any time. A future cleanup slice will resume by id or set a retention policy.

- [Risk] **Auto-compaction and auto-retry events are dropped.** If a session compacts mid-turn (the SDK emits `compaction_start`/`end`), the user sees a silent gap before the next text delta. → **Mitigation:** acceptable for v1; the conversation still proceeds correctly. A future change can surface a `StatusChange("compacting")` line.

- [Trade-off] **Server-side translation hides SDK richness.** Thinking blocks, partial tool results, and tool-execution updates never reach the client. The renderer is "pretty" but lossy. → **Conscious choice:** v1 visual parity with the stub trumps fidelity; we'll layer fidelity in once we know what we want to draw.

- [Trade-off] **No translator-level test for the real SDK.** The unit test feeds hand-crafted SDK-shaped objects, not real SDK output. If pi-agent-core's runtime shape diverges from its TypeScript declarations, the test passes but production breaks. → **Mitigation:** manual end-to-end verification (item 4 in the verification list) catches drift before archive.

## Migration Plan

This is additive code paths only. Rollback = revert the PR. No data migrations, no schema changes, no protocol fixture changes (the on-the-wire `AgentEvent` shape is unchanged because translation is server-side).
