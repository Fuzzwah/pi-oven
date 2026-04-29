## 1. Protocol crate (Rust)

- [ ] 1.1 Add `serde` (with `derive`) and `serde_json` to `crates/pi-oven-protocol/Cargo.toml` if not already present from scaffold-runtime
- [ ] 1.2 In `crates/pi-oven-protocol/src/lib.rs` (or `src/msg.rs`), define a tagged-union `Msg` enum using `#[serde(tag = "type")]`. Initial variants: `Hello { key: String, client_version: String }`, `Welcome { server_version: String }`, `AuthFailed { reason: String }`, `Ping { ts_ms: u64 }`, `Pong { client_ts_ms: u64, server_ts_ms: u64 }`
- [ ] 1.3 Re-export `Msg` from `lib.rs` so consumers can `use pi_oven_protocol::Msg`
- [ ] 1.4 Unit tests in `crates/pi-oven-protocol/tests/` covering: round-trip serialise+deserialise for every variant, `type` field is the tag, unknown `type` deserialises as a tagged error (`serde` returns Err for unknown variants by default)
- [ ] 1.5 Forward-compat test: a Hello with an extra unknown field deserialises successfully (use `#[serde(default)]` or rely on default ignore-unknown behaviour)

## 2. Protocol module (TypeScript)

- [ ] 2.1 Create [packages/pi-oven-server/src/protocol.ts](packages/pi-oven-server/src/protocol.ts) declaring a `Msg` discriminated union mirroring the Rust enum
- [ ] 2.2 Export type guards `isHello(m): m is Hello`, `isPing(m): m is Ping` etc. for safe narrowing in handlers
- [ ] 2.3 Export `decodeMsg(raw: string): Msg | null` that parses JSON, validates `type` is a known variant, returns `null` (and a structured warn log) on unknown / malformed input
- [ ] 2.4 Export `encodeMsg(msg: Msg): string` that JSON-stringifies a typed message
- [ ] 2.5 Vitest unit tests in `packages/pi-oven-server/test/protocol.test.ts`: round-trip every variant, decode rejects unknown `type`, decode rejects malformed JSON
- [ ] 2.6 Cross-language fixture test: hand-craft a Hello JSON string in Rust tests AND in TS tests with the IDENTICAL bytes (including key ordering tolerance — both sides accept any field order); ensure both decode it identically

## 3. Server config extension

- [ ] 3.1 Extend [packages/pi-oven-server/src/config.ts](packages/pi-oven-server/src/config.ts) with `[net]` section: `listen_addr` (default `127.0.0.1:7878`), `shared_key` (no default), `origin_allowlist` (default `[]`), `allow_null_origin` (default `true`)
- [ ] 3.2 Add env-var override `PI_OVEN_SHARED_KEY` for `[net].shared_key`
- [ ] 3.3 Resolution: prefer config file value over env var when both are set
- [ ] 3.4 If neither config nor env provides a non-empty `shared_key`, return a typed error with field name `shared_key` so the boot sequence can log step `"shared_key"` and exit non-zero
- [ ] 3.5 Vitest tests covering: defaults, file-only key, env-only key, file-overrides-env, both-missing throws

## 4. Server WebSocket listener

- [ ] 4.1 Create [packages/pi-oven-server/src/net/server.ts](packages/pi-oven-server/src/net/server.ts) exporting `startListener(opts: { listen_addr, shared_key, origin_allowlist, allow_null_origin, logger }): Promise<{ close(): Promise<void> }>`
- [ ] 4.2 Use `ws.WebSocketServer` with `verifyClient(info, cb)` to enforce the Origin policy: extract `info.origin`, normalise (`null`/missing → `null`), accept localhost / 127.0.0.1 unconditionally, accept `null` iff `allow_null_origin`, otherwise require allowlist match. Reject with HTTP 403 by calling `cb(false, 403, 'forbidden origin')`. Log every rejection with origin + remote address
- [ ] 4.3 On `connection`, set a 5s handshake timer. Listen for the FIRST message: if it's `Hello { key }` and `key === shared_key`, clear the timer, send `Welcome { server_version }`, transition to `authenticated`; if key mismatches, send `AuthFailed { reason: "invalid_key" }` then close with code 4401; if first message is anything else, close with 4401 and reason `"protocol: expected Hello"`. If the timer fires first, close with 4408
- [ ] 4.4 Single-connection invariant: maintain a module-level `Map<sharedKey, WebSocket>` of authenticated sockets. On a new authentication, if a prior socket exists, close it with code 4002 reason `"replaced"` BEFORE accepting the new one
- [ ] 4.5 Heartbeat tracking: on every received frame from an authenticated socket, update `last_seen_at`. A `setInterval` (5s) sweeps authenticated connections and closes any with `last_seen_at` older than 60s using code 4001 reason `"idle_timeout"`
- [ ] 4.6 Pong replies: on receiving `Ping { ts_ms }`, immediately reply `Pong { client_ts_ms: ts_ms, server_ts_ms: Date.now() }`
- [ ] 4.7 Bind error handling: if `WebSocketServer` emits `error` with code `EADDRINUSE` or similar, surface as a typed startup error so the boot sequence can log step `"bind"` and exit non-zero
- [ ] 4.8 `close()` returns a promise that resolves once all connected sockets are closed and the server stops accepting

## 5. Server boot sequence integration

- [ ] 5.1 In [packages/pi-oven-server/src/index.ts](packages/pi-oven-server/src/index.ts), insert a `startListener(...)` step between `migrate(...)` and the `"ready"` log line. Pass the `[net]` config and the root logger
- [ ] 5.2 Extend the `"ready"` log bindings to include `listen_addr` from config
- [ ] 5.3 In SIGINT / SIGTERM handlers, call `listener.close()` BEFORE `db.close()` and lock release so connected clients receive a clean close (code 1001 going-away or our 4001) rather than a TCP RST
- [ ] 5.4 If `startListener` throws, catch in the boot try/catch and log step `"bind"` (or `"shared_key"` from config) before exiting non-zero

## 6. Server integration tests

- [ ] 6.1 [packages/pi-oven-server/test/net.handshake.test.ts](packages/pi-oven-server/test/net.handshake.test.ts): start a real listener on an ephemeral port, connect with `ws` client, send Hello with correct key, expect Welcome
- [ ] 6.2 Same suite: send Hello with wrong key, expect AuthFailed + close 4401
- [ ] 6.3 Same suite: connect and send no frame, expect close 4408 within ~6s
- [ ] 6.4 Same suite: send a non-Hello first frame, expect close 4401
- [ ] 6.5 Origin policy: connect with `Origin: https://evil.example.com`, expect HTTP 403 (no upgrade); connect with no Origin (default in `ws` client) → success; connect with `Origin: http://localhost:5173` → success
- [ ] 6.6 Replaced invariant: open two connections sequentially with the same key; expect the first to receive close 4002 reason "replaced"
- [ ] 6.7 Heartbeat: open a connection, send Ping, assert Pong arrives with matching `client_ts_ms`
- [ ] 6.8 Idle timeout: open a connection, send no frames after Hello/Welcome, advance fake timers (vitest `vi.useFakeTimers()`) past 60s, assert server-initiated close with code 4001

## 7. Client WebSocket transport (Rust)

- [ ] 7.1 Add `tokio-tungstenite` (with `native-tls` or `rustls` feature, but since we're plaintext-only for v1, the default `tokio-rustls`-free build is fine) and `futures-util` to `crates/pi-oven-net/Cargo.toml` if not already declared
- [ ] 7.2 Create [crates/pi-oven-net/src/client.rs](crates/pi-oven-net/src/client.rs) exposing `Client::connect(url: &str, shared_key: &str, client_version: &str) -> Result<Client>` that opens the WebSocket, sends `Hello`, awaits `Welcome` (or returns an `Err` carrying the `AuthFailed` reason / close code)
- [ ] 7.3 Define a `ClientHandle` returned from connect: holds a `tokio::sync::mpsc::Sender<Msg>` for outgoing frames and a `tokio::sync::mpsc::Receiver<Msg>` for incoming
- [ ] 7.4 Spawn a background task driving `tokio::select!` over the socket reader and the outgoing channel; on socket EOF / error, signal connection lost via the receiver closing
- [ ] 7.5 Heartbeat: spawn a `tokio::time::interval(Duration::from_secs(20))` task that sends `Ping { ts_ms }` and tracks the last Pong receipt; if two consecutive pings get no Pong within 20s each, drop the connection
- [ ] 7.6 Re-export `Client`, `ClientHandle`, and connection-state types from `crates/pi-oven-net/src/lib.rs`

## 8. Client reconnect logic

- [ ] 8.1 Create [crates/pi-oven-net/src/reconnect.rs](crates/pi-oven-net/src/reconnect.rs) implementing a wrapper that wraps `Client::connect`, catches close codes, and schedules retries
- [ ] 8.2 Backoff: `min(30, 2u64.pow(attempt)) seconds` with ±25% jitter; reset `attempt` to 0 on every successful Welcome
- [ ] 8.3 Skip reconnect if the close code was 4401 (auth failed) or 4002 (replaced) or 1000 (normal); these are terminal states surfaced to the caller via the receiver channel closing with a typed reason
- [ ] 8.4 Expose the connection state as an enum: `Connecting`, `Authenticated`, `Reconnecting { in_seconds }`, `Failed { reason }` — main loop will render this in the status line eventually
- [ ] 8.5 Unit tests using `tokio::test` and a mock server (`tokio-tungstenite`'s `accept_async` on a `TcpListener::bind("127.0.0.1:0")`): verify backoff schedule, verify no-reconnect on 4401, verify reset on success

## 9. Client wiring (binary)

- [ ] 9.1 In [crates/pi-oven/src/main.rs](crates/pi-oven/src/main.rs), read `PI_OVEN_SERVER_URL` (default `ws://localhost:7878`) and `PI_OVEN_SHARED_KEY` (required; if missing, log error and continue without networking — UI still works for development)
- [ ] 9.2 Spawn the network task using `tokio` runtime alongside winit's event loop. Recommended: `tokio` runtime in a separate thread pumping its own future, communicating with winit thread via `tokio::sync::mpsc` channels
- [ ] 9.3 Log connection state changes at info level: `connecting`, `authenticated`, `reconnecting in Ns`, `disconnected: <reason>`
- [ ] 9.4 Reuse the existing tracing subscriber from scaffold-runtime; add a `target = "pi_oven_net"` filter recommendation in README

## 10. Documentation & manual verification

- [ ] 10.1 Update [README.md](README.md) Development section with: `PI_OVEN_SHARED_KEY` env var requirement, sample `server.toml` `[net]` block, default `listen_addr`, how to override server URL on the client
- [ ] 10.2 Manual verification: `pnpm --filter pi-oven-server dev` starts; `RUST_LOG=pi_oven_net=debug,info cargo run -p pi-oven` logs `connecting → authenticated`; the server NDJSON log shows the upgrade and Hello/Welcome
- [ ] 10.3 Manual verification: relaunch the .app while the first instance is still connected; confirm the first sees `replaced` (close 4002) in its log and the second connects cleanly
- [ ] 10.4 Manual verification: stop the server (`Ctrl+C`); confirm the client logs `disconnected: <reason>` and starts reconnect attempts with growing backoff in the log
- [ ] 10.5 Manual verification: set `PI_OVEN_SHARED_KEY=wrong` on the client; relaunch; confirm `AuthFailed` and NO further reconnect attempts
- [ ] 10.6 Manual verification: deploy the new server build to p330 (`git pull && pnpm install && pnpm --filter pi-oven-server build && systemctl --user restart pi-oven-server`); from the laptop, point `PI_OVEN_SERVER_URL=ws://10.1.1.232:7878` and verify Hello/Welcome over the LAN
