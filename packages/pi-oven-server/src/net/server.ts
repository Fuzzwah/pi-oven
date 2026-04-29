import { WebSocketServer, WebSocket } from "ws";
import type { Logger } from "pino";
import { decodeMsg, encodeMsg } from "../protocol.js";
import type { WorkspaceManager } from "../workspaces/manager.js";

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
  manager?: WorkspaceManager;
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
  const { listen_addr, shared_key, origin_allowlist, allow_null_origin, logger, manager } = opts;
  const { host, port } = parseAddr(listen_addr);

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

  wss.on("error", (err) => {
    logger.error({ err: (err as Error).message }, "WebSocket server error");
  });

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

    const handshakeTimer = setTimeout(() => {
      if (!authenticated_this) {
        ws.close(4408, "handshake_timeout");
      }
    }, HANDSHAKE_TIMEOUT_MS);

    ws.on("message", (data) => {
      if (!authenticated_this) {
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

        const existing = authenticated.get(shared_key);
        if (existing) {
          manager?.setClient(null);
          existing.ws.close(4002, "replaced");
          authenticated.delete(shared_key);
        }

        authenticated.set(shared_key, { ws, lastSeenAt: Date.now() });
        authenticated_this = true;
        manager?.setClient(ws);

        logger.info({ client_version: msg.client_version }, "authenticated");
        ws.send(
          encodeMsg({
            type: "Welcome",
            server_version: SERVER_VERSION,
            workspaces: manager?.getSnapshots() ?? [],
          }),
        );
        return;
      }

      const entry = authenticated.get(shared_key);
      if (entry) entry.lastSeenAt = Date.now();

      const msg = decodeMsg(data.toString());
      if (!msg) return;

      if (msg.type === "Ping") {
        ws.send(encodeMsg({ type: "Pong", client_ts_ms: msg.ts_ms, server_ts_ms: Date.now() }));
        return;
      }

      if (msg.type === "Send") {
        logger.info({ workspace_id: msg.workspace_id, text_len: msg.text.length }, "Send received");
        const session = manager?.getSession(msg.workspace_id);
        if (!session) {
          ws.send(
            encodeMsg({
              type: "ErrorEvent",
              workspace_id: msg.workspace_id,
              reason: "unknown_workspace",
            }),
          );
          return;
        }
        void session.queue(msg.text, msg.queue_mode as "steer" | "followup");
        return;
      }

      if (msg.type === "Abort") {
        const session = manager?.getSession(msg.workspace_id);
        if (!session) return;
        void session.abort();
        return;
      }

      if (msg.type === "Resume") {
        logger.info({ workspace_id: msg.workspace_id, last_seq: msg.last_seq }, "Resume received");
        const session = manager?.getSession(msg.workspace_id);
        if (!session) {
          ws.send(
            encodeMsg({
              type: "ErrorEvent",
              workspace_id: msg.workspace_id,
              reason: "unknown_workspace",
            }),
          );
          return;
        }
        void handleResume(ws, manager!, msg.workspace_id, msg.last_seq);
        return;
      }
    });

    ws.on("close", () => {
      clearTimeout(handshakeTimer);
      const entry = authenticated.get(shared_key);
      if (entry?.ws === ws) {
        authenticated.delete(shared_key);
        manager?.setClient(null);
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

async function handleResume(
  ws: WebSocket,
  manager: WorkspaceManager,
  workspaceId: number,
  lastSeq: number,
): Promise<void> {
  const log = manager.getLog(workspaceId);
  if (!log) return;

  // Collect events from log with seq > lastSeq
  const events: Array<{ seq: number; ts: number; event: unknown }> = [];
  for await (const entry of log.replay(lastSeq)) {
    events.push(entry);
  }

  // Merge ring buffer events (deduplicate by seq, fill any gap between log replay and live)
  const ringEvents = manager.getRingBuffer().filter((e) => e.seq > lastSeq);
  const seenSeqs = new Set(events.map((e) => e.seq));
  for (const re of ringEvents) {
    if (!seenSeqs.has(re.seq)) {
      events.push({ seq: re.seq, ts: 0, event: re.event });
      seenSeqs.add(re.seq);
    }
  }

  // Sort by seq
  events.sort((a, b) => a.seq - b.seq);

  const latestSeq = events.length > 0 ? events[events.length - 1]!.seq : lastSeq;

  ws.send(
    encodeMsg({
      type: "ReplayBatch",
      workspace_id: workspaceId,
      events,
      latest_seq: latestSeq,
    }),
  );
}
