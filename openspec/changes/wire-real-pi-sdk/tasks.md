## 1. Dependency

- [x] 1.1 Add `"@mariozechner/pi-coding-agent": "0.70.6"` to `dependencies` in [packages/pi-oven-server/package.json](../../../packages/pi-oven-server/package.json) (pinned exactly per docs/claude_plan.md §11)
- [x] 1.2 Run `pnpm install` from repo root; verify the package resolves without warnings
- [x] 1.3 Spot-check that `import { createAgentSession, type AgentSessionEvent } from "@mariozechner/pi-coding-agent"` type-checks under `pnpm --filter pi-oven-server build`

## 2. SDK event translator (pure, unit-testable)

- [x] 2.1 In [packages/pi-oven-server/src/workspaces/session.ts](../../../packages/pi-oven-server/src/workspaces/session.ts), define a non-exported type `Translation = { kind: "event"; event: unknown } | { kind: "status"; status: "running" | "idle" } | { kind: "drop" }`
- [x] 2.2 Define and `export function translateSdkEvent(ev: AgentSessionEvent): Translation` implementing the mapping table from the spec (`agent_start`→`status running`, `agent_end`→`status idle`, `message_update` with inner `text_delta`→`event { type: "text_delta", text }`, `tool_execution_start`→`event { type: "tool_call", tool_name, args }`, `tool_execution_end`→`event { type: "tool_result", tool_name, output: stringify(result), exit_code: isError ? 1 : 0 }`)
- [x] 2.3 Stringification rule for `tool_result.output`: if the SDK `result` is a string, pass through; otherwise `JSON.stringify(result)`. Keep the rule local to the translator with a short inline comment.
- [x] 2.4 All other SDK event types return `{ kind: "drop" }` (no logging warning — drops are routine and expected)
- [x] 2.5 Create [packages/pi-oven-server/test/sdk-event-translation.test.ts](../../../packages/pi-oven-server/test/sdk-event-translation.test.ts) with one Vitest case per row of the mapping table plus one "unknown event drops" case and one "tool_result with non-string result is JSON-stringified" case
- [x] 2.6 Run `pnpm --filter pi-oven-server test` — new test passes, no regression in existing tests

## 3. Process env defaults (gotcha 12, best effort)

- [x] 3.1 Replace the existing unused `buildSpawnEnv()` in `session.ts` with `function applyChildProcessEnv(workspaceId: number): void`
- [x] 3.2 The function SHALL set the following keys on `process.env` only when the key is currently `undefined`: `LANG=en_US.UTF-8`, `TZ=UTC` (only if neither set), `EDITOR=true`, `TERM=dumb`, `NO_COLOR=1`, `GIT_TERMINAL_PROMPT=0`, `PI_OVEN_WORKSPACE_ID=String(workspaceId)`
- [x] 3.3 `PATH` is never overwritten — the user's PATH is what we want
- [x] 3.4 Add a unit test `applyChildProcessEnv preserves user-set LANG`: set `process.env.LANG = "de_DE.UTF-8"`, call the function, assert `process.env.LANG === "de_DE.UTF-8"` and `process.env.EDITOR === "true"`. Restore env in `afterEach`.

## 4. AgentSession real-SDK wiring

- [x] 4.1 Add a `cwd: string` constructor parameter to `AgentSession`; store as `private readonly cwd`
- [x] 4.2 Add `private piSession?: PiAgentSession` and `private piUnsubscribe?: () => void` fields
- [x] 4.3 Add `async init(): Promise<void>` method. If `process.env.PI_OVEN_SDK_STUB === "1"`, return immediately (stub path needs no init). Otherwise: call `applyChildProcessEnv(this.workspaceId)`, then `const result = await createAgentSession({ cwd: this.cwd })` (let `agentDir` default to `~/.pi/agent` via `getAgentDir()`)
- [x] 4.4 If `result.model === undefined`, throw `new Error(\`pi SDK has no usable model: \${result.modelFallbackMessage ?? "no auth configured"}\`)` so boot fails fast with a clear message
- [x] 4.5 Assign `this.piSession = result.session`; subscribe to events with `this.piUnsubscribe = this.piSession.subscribe((ev) => this.onPiEvent(ev))`
- [x] 4.6 Implement `private onPiEvent(ev: AgentSessionEvent): void`: switch on `translateSdkEvent(ev).kind` — `"event"` calls `void this.emitEvent(translation.event)`, `"status"` calls `void this.emitStatus(translation.status)`, `"drop"` does nothing
- [x] 4.7 Replace the `throw new Error("PI_OVEN_SDK_STUB not set and real SDK not wired")` in `queue()` with: `if (!this.piSession) throw new Error("AgentSession.init() not called"); await this.piSession.prompt(text, { streamingBehavior: mode === "steer" ? "steer" : "followUp", source: "interactive" });`
- [x] 4.8 Update `abort()`: in the non-stub path, `await this.piSession?.abort()`. Then call `await this.emitStatus("idle")` unconditionally so an explicit Abort always settles status to idle even if the SDK doesn't emit `agent_end` (per spec scenario "explicit abort guarantees idle")
- [x] 4.9 Add `async dispose(): Promise<void>` that calls `this.piUnsubscribe?.()` and `this.piSession?.dispose()`. Not called from anywhere yet — added so a future shutdown-cleanup change has a hook
- [x] 4.10 Keep the existing stub branch (`stubQueue`, `stubTimers`, stub `abort` clearing) untouched

## 5. WorkspaceManager wiring

- [x] 5.1 In [packages/pi-oven-server/src/workspaces/manager.ts](../../../packages/pi-oven-server/src/workspaces/manager.ts), add `import { mkdir } from "node:fs/promises";`
- [x] 5.2 In `init(dataDir)`, compute `const cwd = join(dataDir, "workspaces", String(workspaceId));` and `await mkdir(cwd, { recursive: true, mode: 0o700 });` before constructing `EventLog` (order doesn't strictly matter, but group "filesystem prep for workspace 1" together)
- [x] 5.3 Pass `cwd` as the new third constructor argument to `new AgentSession(workspaceId, log, cwd, { ... })`
- [x] 5.4 After `this.sessions.set(workspaceId, session)`, call `await session.init()` so SDK init failures propagate up through the existing `try/catch` boot wrapper in `index.ts`
- [x] 5.5 Verify in [packages/pi-oven-server/src/index.ts](../../../packages/pi-oven-server/src/index.ts) that `await manager.init(cfg.data_dir)` is already awaited at line 84 — no change needed there

## 6. End-to-end verification

- [ ] 6.1 Stub regression: `PI_OVEN_SDK_STUB=1 pnpm --filter pi-oven-server dev` reaches `"ready"` log line; existing E2E flow (Send → text deltas → Abort → idle) behaves identically to before
- [ ] 6.2 Real boot success: `pnpm --filter pi-oven-server dev` (no stub flag) reaches `"ready"` with no errors against the user's existing `~/.pi/agent/auth.json` + `settings.json`
- [ ] 6.3 Real boot failure path: temporarily rename `~/.pi/agent/auth.json` aside, start the server, confirm boot fails with `step: "workspace_manager"` and a message containing "no usable model" or the SDK's fallback message; restore the file
- [ ] 6.4 Manual end-to-end with crossterm client: server in one terminal, `cargo run -p pi-oven --no-default-features --features dev-crossterm` in another. Type "list the files in the current directory" + Enter. Expect: `AgentStatus running` → assistant text streaming in → at least one `tool_call`/`tool_result` pair → final assistant text → `AgentStatus idle`
- [ ] 6.5 Manual abort: with the same setup, send a longer prompt and press Escape mid-stream. Expect status returns to `idle` cleanly
- [ ] 6.6 NDJSON sanity: `tail -n 5 ~/.pi-oven/events/1/*.ndjson` after the run shows only `text_delta`, `tool_call`, `tool_result` events — no `turn_*`, `message_*`, `queue_update`, etc. (proves translation is what hits disk, not raw SDK passthrough)
- [ ] 6.7 Reconnect replay: kill the client during an active stream, relaunch. Conversation pane replays everything up to the server's last seq, then continues live
- [ ] 6.8 cwd verification: `ls -la ~/.pi-oven/workspaces/1` exists with mode `0700`
