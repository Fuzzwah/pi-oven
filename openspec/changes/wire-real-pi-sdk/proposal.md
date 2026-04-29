## Why

The live-session change archived with the pi SDK still stubbed: today, [packages/pi-oven-server/src/workspaces/session.ts](packages/pi-oven-server/src/workspaces/session.ts) emits canned events when `PI_OVEN_SDK_STUB=1` and *throws* otherwise. Everything around it (NDJSON event log, ring buffer, `Resume`/`ReplayBatch`, fan-out, conversation pane) is ready and proven against the stub. This change closes the gap so that running the server without the stub flag drives a real `@mariozechner/pi-coding-agent` session through the existing pipeline — turning the project from "rendered scaffolding wired to a fake agent" into "an actual usable client for one hardcoded workspace."

## What Changes

- Add `@mariozechner/pi-coding-agent` (pinned at `0.70.6`) as a server dependency.
- Replace the throw in `AgentSession.queue` with a real `createAgentSession({ cwd, agentDir })` call. Lazy-init the SDK session in a new `AgentSession.init()` method, called eagerly from `WorkspaceManager.init()` so boot fails fast on missing auth or missing usable model.
- Subscribe to the SDK's `AgentSessionEvent` stream and **translate** events server-side into the existing simple shapes the renderer already understands (`text_delta { text }`, `tool_call { tool_name, args }`, `tool_result { tool_name, output, exit_code }`). SDK lifecycle events (`agent_start`, `agent_end`) drive `AgentStatus running`/`idle`. Unrelated SDK events (`turn_*`, `message_*`, `queue_update`, `compaction_*`, `auto_retry_*`, `session_info_changed`) are dropped to keep the timeline clean.
- Compute `cwd` for hardcoded `workspace_id=1` as `<data_dir>/workspaces/1` (created with mode `0700`). No new config field — derived from existing `data_dir`.
- Apply gotcha-12 environment defensively to `process.env` (only-if-unset) before the first `createAgentSession` call: `LANG`, `TZ`, `EDITOR=true`, `TERM=dumb`, `NO_COLOR=1`, `GIT_TERMINAL_PROMPT=0`, `PI_OVEN_WORKSPACE_ID`. The pi SDK runs in-process, so its bash tool inherits `process.env`; per-tool env injection via custom `BashOperations` is a future slice.
- Keep `PI_OVEN_SDK_STUB=1` working unchanged — existing tests and offline development continue to use the stub path.
- Add a unit test for the SDK-event → protocol-event translator covering every row of the mapping table.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-session`: clarify that the server SHALL translate pi SDK `AgentSessionEvent` payloads into the existing simple `AgentEvent` shape before logging and fan-out, rather than passing SDK events through verbatim. Documents the translator contract, the dropped event types, and the fact that `AgentStatus` transitions are derived from `agent_start`/`agent_end` rather than synthesized.

## Impact

- **Modified code**:
  - [packages/pi-oven-server/src/workspaces/session.ts](packages/pi-oven-server/src/workspaces/session.ts) — bulk of the change. New async `init()`, real-SDK branch in `queue`/`abort`, `dispose`, in-file `translateSdkEvent` pure function, side-effecting `applyChildProcessEnv`.
  - [packages/pi-oven-server/src/workspaces/manager.ts](packages/pi-oven-server/src/workspaces/manager.ts) — derive `cwd`, mkdir it `0700`, pass to `AgentSession`, await `session.init()`.
  - [packages/pi-oven-server/package.json](packages/pi-oven-server/package.json) — add pinned dep.
- **New code**:
  - [packages/pi-oven-server/test/sdk-event-translation.test.ts](packages/pi-oven-server/test/sdk-event-translation.test.ts) — unit test for the translator.
- **Dependencies**: `@mariozechner/pi-coding-agent@0.70.6` (and its transitive tree, including `@mariozechner/pi-agent-core`, `@mariozechner/pi-ai`).
- **Required user state**: `~/.pi/agent/auth.json` and `~/.pi/agent/settings.json` (already present on dev box). Server boot fails fast with a clear error if `result.model` is `undefined`.
- **Out of scope** (deliberately deferred):
  - Multi-workspace, picker UI, worktree creation. Still single hardcoded workspace.
  - Attachments / multimodal content.
  - Custom `BashOperations` for true per-tool env isolation. Process-level env is best-effort.
  - Resuming the same pi session across server restarts. A fresh session file is created in `~/.pi/agent/sessions/` each boot.
  - Enriching the renderer to display thinking blocks, partial tool results, or other SDK-only event types. Those land in a future change once we have a UX story for them.
