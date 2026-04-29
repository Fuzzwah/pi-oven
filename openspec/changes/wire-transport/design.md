## Context

Slice 1 of the roadmap (`docs/claude_plan.md`) split into two halves: the runtime scaffold (now landed as `2026-04-29-scaffold-runtime`) and the wire that connects client to server. The scaffold deliberately stubbed `pi-oven-net`, `pi-oven-protocol`, and the server's network module so the foundational concerns (build system, SQLite, structured logging, key capture) could be validated in isolation. This change closes the gap.

The plan dictates the high-level shape (`docs/claude_plan.md` lines 262–308): single WebSocket, JSON, shared-key handshake, server side bound after migrations succeed. What it does NOT pin down is the framing details, heartbeat cadence, Origin policy, reconnect strategy, or how `null` Origin from a bundled `.app` plays with the allowlist. Those are this design's job.

Constraint reminder: pi-oven is **deliberately single-user**. We do not need session tokens, multi-tenant isolation, OAuth, or per-connection ACLs. One shared key. One client. The server may receive multiple connection attempts (e.g., user opens the .app twice), but only one wins.

## Goals / Non-Goals

**Goals:**
- Establish a tested, authenticated WebSocket between client and server.
- Define the wire envelope shape (`Msg` tagged union) so future message types just add variants.
- Heartbeat that detects half-open connections within ~40 seconds (covers macOS lid-close / network handoff).
- Origin allowlist that rejects browser-tab attacks while accepting bundled `.app` clients (which send no Origin).
- Boot sequence amendment: listener starts only after migrations succeed.
- A round-trip integration test on each side that uses a real WebSocket (not mocks).

**Non-Goals:**
- pi SDK integration — this change carries no agent events. `Send`, `AgentEvent`, `Resume`, `ReplayBatch` are deferred.
- Project / workspace messages.
- Binary frames (image upload) — JSON only for now.
- Replay-on-reconnect — there are no buffered events to replay yet. The reconnect logic in this change is a clean reconnect (re-handshake), not an event-stream resume.
- TLS (`wss://`). LAN plaintext for v1; bring-your-own-reverse-proxy for exposed deployments.
- Multi-client support. Second connection from the same shared key wins; the prior is closed (4002 `replaced`).
- Persistent client config. Shared key + server URL come from env vars for now; a config file for the client is a later change.

## Decisions

### D1. WebSocket libraries: `ws` (server) + `tokio-tungstenite` (client)

Both are already declared as deps in scaffold-runtime; no new top-level adds. `ws` is the de-facto Node WebSocket library, used by the pi codebase and well-understood operationally. `tokio-tungstenite` is the Rust default; it integrates cleanly with `tokio::select!` for concurrent read/write loops.

**Alternatives considered:** `uWebSockets.js` (faster, but C++ binary deps complicate cross-platform server bootstrap), Engine.IO / Socket.IO (we want plain WebSockets, not a higher-level framework), `axum`'s built-in WS (we're not running a Rust server, that's hypothetical).

### D2. Tagged-union message envelope (`Msg`) with serde `tag = "type"`

Every message has a `type: "Hello" | "Welcome" | …` discriminator at the JSON top level. Rust uses `#[derive(Serialize, Deserialize)] #[serde(tag = "type")]` on a single `Msg` enum; TypeScript uses a discriminated union with a literal `type` field. Mirror naming **exactly** between the two sides — the wire is the contract.

**Alternatives considered:** Adjacently-tagged (`{ "type": "...", "data": {...} }`) — adds a layer of nesting for marginal type-safety gain. Externally-tagged (`{ "Hello": {...} }`) — serde default for enums but harder to read in logs. Internally-tagged (chosen) is the standard for tagged JSON unions and reads naturally in NDJSON server logs.

### D3. Shared-key auth: `Hello` is the FIRST frame; server idle-timeouts handshake at 5s

Client sends `Hello { key, client_version }` immediately after the WebSocket upgrade completes. Server validates synchronously: if key matches `shared_key`, reply `Welcome { server_version }` and transition the connection to "active". If not, send `AuthFailed { reason: "invalid_key" }` then close with WebSocket code `4401`. If the client doesn't send anything within 5s of the upgrade, server closes with code `4408` (timeout).

**Alternatives considered:** Header-based auth (`Authorization: Bearer <key>` on the upgrade request) — works for code-driven clients but is awkward for browser-based testing and harder to log. Session tokens issued via separate HTTP — adds an endpoint and state for a single-user system. Mutual TLS — overkill for LAN; later option for exposed deployments.

**WebSocket close codes used here:**
| Code | Meaning |
|------|---------|
| 1000 | Normal close (graceful shutdown) |
| 4001 | Server shutting down (client should retry with backoff) |
| 4002 | Replaced by newer connection from same key |
| 4401 | Auth failed (client should NOT retry — operator must fix the key) |
| 4408 | Handshake timeout |

The 4xxx range is reserved for application use per RFC 6455 §7.4.2.

### D4. Origin allowlist with explicit `null` opt-in

Server's WebSocket upgrade handler reads the `Origin` header. If the request comes from `localhost`/`127.0.0.1` or matches an entry in `origin_allowlist`, accept. If `Origin` is absent or `null` (bundled `.app` does this — see WHATWG fetch spec on opaque origin), accept iff config sets `allow_null_origin: true` (default `true`). Otherwise, reject the upgrade with HTTP 403 before WebSocket negotiation completes.

**Why care?** Even on LAN, a browser tab at `evil.example.com` could open `ws://localhost:7878/` and pre-authenticate by guessing the shared key. Origin policy is the cheap defence-in-depth that complements the key.

**Alternatives considered:** Strict same-origin only (rejects `.app` because it sends no Origin — fails our primary use case). No Origin checking (single line of defence on a guessable shared key — accepted by some single-user tools, rejected here because the cost is one config line).

### D5. Application-level heartbeat: 20s interval, 2 missed = dead

After the handshake, client sends `Ping { ts_ms }` every 20 seconds; server replies `Pong { client_ts_ms, server_ts_ms }`. If the client misses two consecutive pongs (no response within 40s of the first ping), the client closes the connection and schedules a reconnect. Symmetrically, the server tracks last-seen-from-client; if no frame (any frame) for 60s, server closes with code 4001.

**Why app-level vs RFC 6455 ping/pong?** Two reasons:
1. Mid-stack proxies sometimes strip control frames — `ping` opcode `0x9` is opaque, `Ping` JSON is observable in logs and tests.
2. Future round-trip telemetry (`ts_ms` echo lets us measure RTT cheaply, eventually feeding a connection-quality indicator in the UI).

**Alternatives considered:** `tokio-tungstenite`'s built-in ping (works fine, but invisible in NDJSON logs), longer interval (60s — too slow to detect macOS sleep, which is the primary failure mode), shorter interval (5s — wastes battery, log noise).

### D6. Reconnect: exponential backoff 1s → 30s, jittered, no replay

On any close that isn't 4401 (auth failed) or 1000 (normal close after `Cmd+W` quit), client schedules a reconnect after `min(30, 2^attempt)` seconds with ±25% jitter. On successful reconnect, client re-runs the handshake; **no resume / replay** in this change because there are no buffered events yet. The eventual `Resume { last_seq }` design (per `claude_plan.md`) lands with the agent-event slice that introduces seq-numbered events to replay.

**Alternatives considered:** Linear backoff (won't ramp away during prolonged outages), capped retries (counterproductive — user wants the client to just keep trying), instant retry (storms the server during real outages).

### D7. Single-connection-per-key invariant: 4002 `replaced`

If a second client connects with a valid `Hello` while another connection is already authenticated, the server closes the **older** connection with code `4002` and accepts the new one. Rationale: matches user expectation when reopening the .app after a crash — the old socket is dead-but-OS-still-thinks-it's-open; user shouldn't have to wait for the server's idle timeout.

**Alternatives considered:** Reject the new connection (4001) — feels broken to the user who just clicked the dock icon. Allow both — single-user model has no use for it; complicates broadcast routing later.

### D8. Listener starts after migrations, fails closed if migrations fail

The server's existing boot sequence (per `server-runtime` spec) is loadConfig → acquireLock → initLogger → openDb → migrate → log "ready". This change inserts startListener between migrate and "ready". If startListener fails (port in use, address invalid), the server logs an error step name and exits non-zero — same shape as the existing failure handling. No half-started state.

### D9. Shared key sourcing: file > env, fail-fast if missing

`shared_key` resolution order: `~/.pi-oven/server.toml` `[net].shared_key` (preferred — file mode `0600` already enforced), then `PI_OVEN_SHARED_KEY` env (convenient for development). If neither is set, server logs an error and exits — does NOT auto-generate a key, because then client and server would never agree on it.

For client: `PI_OVEN_SHARED_KEY` env var only for now (no client config file in this change).

**Alternatives considered:** Auto-generate on first run and write to both ends — split-brain hazard since the client and server live on different hosts. Hard-coded key — security joke. Prompt on first run — won't work for the headless server.

### D10. Server URL: `PI_OVEN_SERVER_URL` env, default `ws://localhost:7878`

Default targets a local server for dev. Production users (LAN deployment) override with `ws://<host>:<port>`. We don't sniff Bonjour, mDNS, or pi.dev's discovery — single user, single config line.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| `null` Origin acceptance is also what a Chrome extension running with `host_permissions` could send. | The shared key remains the load-bearing auth. Origin allowlist is defence-in-depth; a properly configured allowlist (default accepts only `null` and `localhost`) keeps that surface bounded. Documented in README. |
| Shared key in `PI_OVEN_SHARED_KEY` env var leaks via `ps -E` on shared hosts. | Single-user, single-host server (the user's own LAN box). Documented preference: put the key in `server.toml` (`0600`) over env. Future change can add OS keychain integration. |
| TCP send-buffer fills on a slow client and the server blocks. | For this change, no broadcast traffic — only handshake + heartbeat. Backpressure design is deferred to the agent-event slice that actually emits high-volume traffic. |
| Reconnect storms after a server restart with 100s of clients. | Single-user system — at most one client ever reconnects. Jittered backoff covers the corner case (e.g., a script restarting the .app in a loop). |
| Heartbeat at 20s burns laptop battery during sleep. | macOS suspends timers on sleep, so the next Ping fires on wake. Net effect: connection looks idle, server times out at 60s, client reconnects on wake. This is the desired behaviour. |
| Single-connection invariant racing with reconnect: client A sees its own old socket closed, retries fast, races with client B. | 4002 `replaced` close code is permanent — client checks the close code and does NOT retry on 4002. Otherwise treats 4001/network-error as transient and backs off. |

## Migration Plan

This is additive — no data migration, no protocol-version negotiation needed (yet). Deploy steps:
1. Install dependencies (`pnpm install` picks up `ws` from scaffold-runtime; `cargo build` picks up `tokio-tungstenite`).
2. Set `PI_OVEN_SHARED_KEY` (or write to `server.toml`) on the server host.
3. Restart the server — listener starts after migrations.
4. Set `PI_OVEN_SERVER_URL` and `PI_OVEN_SHARED_KEY` on the client; launch the .app.
5. Verify `Welcome` appears in client debug logs and a `connected` line appears in server NDJSON.

Rollback: this slice is independent — pre-existing scaffold-runtime continues to work without it; the client just doesn't connect.

## Open Questions

None blocking. Items deferred to later changes (and not part of this design):
- Whether to bundle a CLI helper to print a freshly generated shared key (`pi-oven-server keygen`). Falls out of the config-file UX, deferrable.
- Whether `tokio-tungstenite` or a thin layer over it should also handle the eventual binary-frame upload (image attachments). Decided when that change starts; nothing in this change precludes either path.
