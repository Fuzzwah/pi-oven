## Why

The runtime scaffold builds and runs end-to-end, but the Rust client and Node/TS server have no way to talk to each other — `pi-oven-net` is an empty crate, the server has no listener, and `pi-oven-protocol` ships a placeholder `Msg`. Every later slice (multi-workspace, agent round-trip, attachments, theming) assumes the wire is live, so the transport is the single keystone unblocking the rest of the roadmap.

## What Changes

- **New: handshake message vocabulary** in `pi-oven-protocol` — `Hello { key, client_version }`, `Welcome { server_version }`, `AuthFailed { reason }`, plus a tagged-union `Msg` envelope using serde's `#[serde(tag = "type")]` convention. Mirror the same shapes in `packages/pi-oven-server/src/protocol.ts`.
- **New: WebSocket server** at `packages/pi-oven-server/src/net/server.ts` using `ws`. Binds to a configurable address; auth via shared key in the first frame (rejects after `AuthFailed`); rejects upgrade requests whose `Origin` is not on a configured allowlist.
- **New: WebSocket client** in `pi-oven-net` using `tokio-tungstenite`. Connects on app startup; sends `Hello` first frame; consumes `Welcome` to confirm the session; logs `AuthFailed` and stays disconnected with a clear surfacing path for the UI.
- **New: heartbeat** — application-level `Ping`/`Pong` every 20s on both sides, using monotonic timestamps. Connection is considered dead after two missed pongs; client schedules a reconnect attempt with exponential backoff (1s → 30s cap).
- **New: server config keys** — `listen_addr` (default `127.0.0.1:7878`), `shared_key` (required; loaded from `~/.pi-oven/server.toml` or `PI_OVEN_SHARED_KEY` env), `origin_allowlist` (default `["http://localhost", "https://localhost"]` — bundled `.app` clients send `null` Origin which is also accepted).
- **New: client config** — server URL and shared key flow into the binary via env (`PI_OVEN_SERVER_URL`, `PI_OVEN_SHARED_KEY`) for the prototype; persistent client config lands in a later change.
- **Modified: server boot sequence** — the listener starts AFTER `migrate()` returns successfully, before logging `"ready"`. A half-migrated DB is never visible on the wire.
- **Out of scope (deferred to later changes):** pi SDK integration, agent event passthrough, project / workspace messages, attachments, replay-on-reconnect (no buffered events yet to replay), TLS / wss (LAN-plaintext for v1).

## Capabilities

### New Capabilities
- `wire-transport`: the single WebSocket between client and server — handshake message vocabulary, shared-key auth, Origin policy, application-level heartbeat, connection lifecycle on both ends.

### Modified Capabilities
- `server-runtime`: boot sequence is extended with one new step (start the WebSocket listener) inserted between `migrate()` and the `"ready"` log line.

## Impact

- **New crates / modules:** `pi-oven-protocol/src/msg.rs` (real types), `pi-oven-net/src/client.rs`, `pi-oven-net/src/heartbeat.rs`, `packages/pi-oven-server/src/net/server.ts`, `packages/pi-oven-server/src/protocol.ts`.
- **Server boot sequence** gains a step — the existing `server-runtime` spec is amended to require listener start after migration.
- **Config surface grows:** `listen_addr`, `shared_key`, `origin_allowlist` on the server; `PI_OVEN_SERVER_URL` + `PI_OVEN_SHARED_KEY` env vars on the client.
- **Dependencies:** `ws` (server), `tokio-tungstenite` (client) — both already declared as deps in scaffold-runtime; no new top-level package adds.
- **Test surface:** Vitest suite gets a transport integration test (real WebSocket, real auth handshake, real Origin reject); Rust gets a smoke test that round-trips Hello/Welcome against a stub server.
- **Not affected:** SQLite schema, migrations, lock file, log file format, ratatui widgets, render pipeline.
