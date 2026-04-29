import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import WebSocket from "ws";
import pino from "pino";
import { startListener, type ListenerHandle } from "../src/net/server.js";
import { encodeMsg, decodeMsg } from "../src/protocol.js";

const SHARED_KEY = "test-shared-key";
const logger = pino({ level: "silent" });

function makeOpts(overrides: Partial<Parameters<typeof startListener>[0]> = {}) {
  return {
    listen_addr: "127.0.0.1:0",
    shared_key: SHARED_KEY,
    origin_allowlist: [] as string[],
    allow_null_origin: true,
    logger,
    ...overrides,
  };
}

function connect(port: number, opts: { headers?: Record<string, string> } = {}): WebSocket {
  return new WebSocket(`ws://127.0.0.1:${port}`, { headers: opts.headers ?? {} });
}

function wsOpen(ws: WebSocket): Promise<void> {
  return new Promise((resolve, reject) => {
    ws.once("open", resolve);
    ws.once("error", reject);
  });
}

function wsMessage(ws: WebSocket): Promise<string> {
  return new Promise((resolve, reject) => {
    ws.once("message", (data) => resolve(data.toString()));
    ws.once("error", reject);
    ws.once("close", () => reject(new Error("closed before message")));
  });
}

function wsClose(ws: WebSocket): Promise<{ code: number; reason: string }> {
  return new Promise((resolve) => {
    ws.once("close", (code, reason) => resolve({ code, reason: reason.toString() }));
  });
}

let handle: ListenerHandle;

beforeEach(async () => {
  handle = await startListener(makeOpts());
});

afterEach(async () => {
  await handle.close();
});

// ─── 6.1 Happy path ──────────────────────────────────────────────────────────

describe("6.1 – successful handshake", () => {
  it("sends Welcome after correct Hello", async () => {
    const ws = connect(handle.port);
    await wsOpen(ws);
    ws.send(encodeMsg({ type: "Hello", key: SHARED_KEY, client_version: "0.1.0" }));
    const raw = await wsMessage(ws);
    const msg = decodeMsg(raw);
    expect(msg?.type).toBe("Welcome");
    ws.close();
  });
});

// ─── 6.2 Wrong key ───────────────────────────────────────────────────────────

describe("6.2 – wrong key", () => {
  it("sends AuthFailed then closes with 4401", async () => {
    const ws = connect(handle.port);
    await wsOpen(ws);
    const closeProm = wsClose(ws);
    ws.send(encodeMsg({ type: "Hello", key: "wrong-key", client_version: "0.0.0" }));
    const raw = await wsMessage(ws);
    expect(decodeMsg(raw)?.type).toBe("AuthFailed");
    const { code } = await closeProm;
    expect(code).toBe(4401);
  });
});

// ─── 6.3 No frame → 4408 ─────────────────────────────────────────────────────

describe("6.3 – handshake timeout", () => {
  it("closes with 4408 if no frame sent within 5s", async () => {
    const ws = connect(handle.port);
    await wsOpen(ws);
    const { code } = await wsClose(ws);
    expect(code).toBe(4408);
  }, 7_000);
});

// ─── 6.4 Non-Hello first frame ───────────────────────────────────────────────

describe("6.4 – non-Hello first frame", () => {
  it("closes with 4401", async () => {
    const ws = connect(handle.port);
    await wsOpen(ws);
    const closeProm = wsClose(ws);
    ws.send(encodeMsg({ type: "Ping", ts_ms: Date.now() }));
    const { code } = await closeProm;
    expect(code).toBe(4401);
  });
});

// ─── 6.5 Origin policy ───────────────────────────────────────────────────────

describe("6.5 – origin policy", () => {
  it("rejects untrusted origin with HTTP 403", async () => {
    // `ws` client gets an 'error' (unexpected server response) on 403 upgrade reject.
    const rejectedHandle = await startListener(
      makeOpts({ allow_null_origin: false, origin_allowlist: [] }),
    );
    try {
      const ws = new WebSocket(`ws://127.0.0.1:${rejectedHandle.port}`, {
        headers: { Origin: "https://evil.example.com" },
      });
      await new Promise<void>((resolve, reject) => {
        ws.once("unexpected-response", (_req, res) => {
          expect(res.statusCode).toBe(403);
          resolve();
        });
        ws.once("open", () => reject(new Error("should have been rejected")));
        ws.once("error", (err) => {
          // ws raises an error for 403; check message contains status
          expect(err.message).toMatch(/403|Unexpected server response/i);
          resolve();
        });
      });
    } finally {
      await rejectedHandle.close();
    }
  });

  it("accepts connection with no Origin header (null origin, allow_null_origin=true)", async () => {
    // ws client sends no Origin by default when constructed with a plain ws:// URL
    // and no explicit Origin header.
    const ws = new WebSocket(`ws://127.0.0.1:${handle.port}`);
    await wsOpen(ws);
    ws.send(encodeMsg({ type: "Hello", key: SHARED_KEY, client_version: "0.0.0" }));
    const raw = await wsMessage(ws);
    expect(decodeMsg(raw)?.type).toBe("Welcome");
    ws.close();
  });

  it("accepts localhost origin unconditionally", async () => {
    const strictHandle = await startListener(
      makeOpts({ allow_null_origin: false, origin_allowlist: [] }),
    );
    try {
      const ws = new WebSocket(`ws://127.0.0.1:${strictHandle.port}`, {
        headers: { Origin: "http://localhost:5173" },
      });
      await wsOpen(ws);
      ws.send(encodeMsg({ type: "Hello", key: SHARED_KEY, client_version: "0.0.0" }));
      const raw = await wsMessage(ws);
      expect(decodeMsg(raw)?.type).toBe("Welcome");
      ws.close();
    } finally {
      await strictHandle.close();
    }
  });
});

// ─── 6.6 Replaced invariant ──────────────────────────────────────────────────

describe("6.6 – replaced invariant", () => {
  it("first connection receives 4002 when second authenticates", async () => {
    const ws1 = connect(handle.port);
    await wsOpen(ws1);
    ws1.send(encodeMsg({ type: "Hello", key: SHARED_KEY, client_version: "0.0.0" }));
    const welcome1 = await wsMessage(ws1);
    expect(decodeMsg(welcome1)?.type).toBe("Welcome");

    const closeProm1 = wsClose(ws1);

    const ws2 = connect(handle.port);
    await wsOpen(ws2);
    ws2.send(encodeMsg({ type: "Hello", key: SHARED_KEY, client_version: "0.0.0" }));
    const welcome2 = await wsMessage(ws2);
    expect(decodeMsg(welcome2)?.type).toBe("Welcome");

    const { code, reason } = await closeProm1;
    expect(code).toBe(4002);
    expect(reason).toMatch(/replaced/i);

    ws2.close();
  });
});

// ─── 6.7 Heartbeat ───────────────────────────────────────────────────────────

describe("6.7 – heartbeat Ping/Pong", () => {
  it("server replies Pong with matching client_ts_ms", async () => {
    const ws = connect(handle.port);
    await wsOpen(ws);
    ws.send(encodeMsg({ type: "Hello", key: SHARED_KEY, client_version: "0.0.0" }));
    await wsMessage(ws); // Welcome

    const ts = Date.now();
    ws.send(encodeMsg({ type: "Ping", ts_ms: ts }));
    const raw = await wsMessage(ws);
    const msg = decodeMsg(raw);
    expect(msg?.type).toBe("Pong");
    if (msg?.type === "Pong") {
      expect(msg.client_ts_ms).toBe(ts);
      expect(msg.server_ts_ms).toBeGreaterThan(0);
    }
    ws.close();
  });
});

// ─── 6.8 Idle timeout ────────────────────────────────────────────────────────

describe("6.8 – idle timeout (fake timers)", () => {
  it("closes authenticated socket with 4001 after 60s idle", async () => {
    vi.useFakeTimers();
    const ws = connect(handle.port);
    await wsOpen(ws);
    ws.send(encodeMsg({ type: "Hello", key: SHARED_KEY, client_version: "0.0.0" }));
    await wsMessage(ws); // Welcome

    const closeProm = wsClose(ws);

    // Advance past IDLE_TIMEOUT_MS (60s) + a heartbeat sweep interval (5s).
    await vi.advanceTimersByTimeAsync(66_000);

    const { code } = await closeProm;
    expect(code).toBe(4001);

    vi.useRealTimers();
  });
});
