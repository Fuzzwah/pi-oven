import { WebSocketServer, WebSocket } from "ws";
import type { Logger } from "pino";
import { decodeMsg, encodeMsg } from "../protocol.js";

const SERVER_VERSION = "0.0.0";
const HANDSHAKE_TIMEOUT_MS = 5_000;
const IDLE_TIMEOUT_MS = 60_000;
const HEARTBEAT_SWEEP_MS = 5_000;

export interface ListenerOpts {
  listen_addr: string;
  shared_key: string;
  origin_allowlist: string[];
  allow_null_origin: boolean;
  logger: Logger;
}

export interface ListenerHandle {
  close(): Promise<void>;
  /** Actual bound port (useful when listen_addr used port 0). */
  port: number;
}

function parseAddr(listen_addr: string): { host: string; port: number } {
  const idx = listen_addr.lastIndexOf(":");
  return {
    host: listen_addr.slice(0, idx),
    port: parseInt(listen_addr.slice(idx + 1), 10),
  };
}

function isAllowedOrigin(
  origin: string | null,
  allowlist: string[],
  allow_null_origin: boolean,
): boolean {
  if (origin === null) return allow_null_origin;
  try {
    const url = new URL(origin);
    if (url.hostname === "localhost" || url.hostname === "127.0.0.1") return true;
  } catch {
    // unparseable origin — fall through to allowlist check
  }
  return allowlist.includes(origin);
}

interface AuthenticatedSocket {
  ws: WebSocket;
  lastSeenAt: number;
}

export async function startListener(opts: ListenerOpts): Promise<ListenerHandle> {
  const { listen_addr, shared_key, origin_allowlist, allow_null_origin, logger } = opts;
  const { host, port } = parseAddr(listen_addr);

  // Module-level map of authenticated sockets keyed by shared_key (task 4.4).
  // There is only one shared_key in this single-user system, but the structure
  // generalises cleanly and matches the spec.
  const authenticated = new Map<string, AuthenticatedSocket>();

  const wss = await new Promise<WebSocketServer>((resolve, reject) => {
    const server = new WebSocketServer({
      host,
      port,
      verifyClient(info, cb) {
        const rawOrigin = info.req.headers["origin"];
        const origin =
          rawOrigin === undefined || rawOrigin === "null" ? null : rawOrigin;

        if (!isAllowedOrigin(origin, origin_allowlist, allow_null_origin)) {
          logger.warn(
            { origin, remoteAddress: info.req.socket.remoteAddress },
            "rejected upgrade: forbidden origin",
          );
          cb(false, 403, "forbidden origin");
          return;
        }
        cb(true);
      },
    });

    server.once("listening", () => {
      server.removeListener("error", reject);
      resolve(server);
    });
    server.once("error", reject);
  });

  // Ongoing server-level error handler (post-bind).
  wss.on("error", (err) => {
    logger.error({ err: (err as Error).message }, "WebSocket server error");
  });

  // Heartbeat sweep: close connections silent for more than IDLE_TIMEOUT_MS (task 4.5).
  const heartbeatInterval = setInterval(() => {
    const now = Date.now();
    for (const [key, entry] of authenticated) {
      if (now - entry.lastSeenAt > IDLE_TIMEOUT_MS) {
        logger.info({ key: key.slice(0, 8) + "…" }, "closing idle connection");
        authenticated.delete(key);
        entry.ws.close(4001, "idle_timeout");
      }
    }
  }, HEARTBEAT_SWEEP_MS);

  wss.on("connection", (ws) => {
    let authenticated_this = false;

    // 5s handshake timer (task 4.3).
    const handshakeTimer = setTimeout(() => {
      if (!authenticated_this) {
        ws.close(4408, "handshake_timeout");
      }
    }, HANDSHAKE_TIMEOUT_MS);

    ws.on("message", (data) => {
      if (!authenticated_this) {
        // First message — handshake path.
        clearTimeout(handshakeTimer);

        const msg = decodeMsg(data.toString());

        if (!msg || msg.type !== "Hello") {
          ws.close(4401, "protocol: expected Hello");
          return;
        }

        if (msg.key !== shared_key) {
          ws.send(encodeMsg({ type: "AuthFailed", reason: "invalid_key" }));
          ws.close(4401, "auth_failed");
          return;
        }

        // Single-connection invariant: replace any existing authenticated socket (task 4.4).
        const existing = authenticated.get(shared_key);
        if (existing) {
          existing.ws.close(4002, "replaced");
          authenticated.delete(shared_key);
        }

        authenticated.set(shared_key, { ws, lastSeenAt: Date.now() });
        authenticated_this = true;

        logger.info({ client_version: msg.client_version }, "authenticated");
        ws.send(encodeMsg({ type: "Welcome", server_version: SERVER_VERSION }));
        return;
      }

      // Authenticated path — update last_seen and handle messages (tasks 4.5, 4.6).
      const entry = authenticated.get(shared_key);
      if (entry) entry.lastSeenAt = Date.now();

      const msg = decodeMsg(data.toString());
      if (!msg) return;

      if (msg.type === "Ping") {
        ws.send(encodeMsg({ type: "Pong", client_ts_ms: msg.ts_ms, server_ts_ms: Date.now() }));
      }
    });

    ws.on("close", () => {
      clearTimeout(handshakeTimer);
      const entry = authenticated.get(shared_key);
      if (entry?.ws === ws) {
        authenticated.delete(shared_key);
      }
    });
  });

  const addr = wss.address() as { port: number };

  return {
    port: addr.port,

    close(): Promise<void> {
      clearInterval(heartbeatInterval);
      return new Promise((resolve, reject) => {
        for (const entry of authenticated.values()) {
          entry.ws.close(1001, "server_shutdown");
        }
        authenticated.clear();
        wss.close((err) => {
          if (err) reject(err);
          else resolve();
        });
      });
    },
  };
}
