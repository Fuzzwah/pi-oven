import { join } from "node:path";
import type { WebSocket } from "ws";
import { AgentSession } from "./session.js";
import { EventLog } from "./events/log.js";
import type {
  AgentEvent,
  AgentStatus,
  WorkspaceSnapshot,
} from "../protocol.js";
import { encodeMsg } from "../protocol.js";

const RING_CAP = 500;

export class WorkspaceManager {
  private sessions = new Map<number, AgentSession>();
  private logs = new Map<number, EventLog>();
  private statusMap = new Map<number, "running" | "idle">();
  private ring: AgentEvent[] = [];
  private connectedWs: WebSocket | null = null;

  async init(dataDir: string): Promise<void> {
    const workspaceId = 1;
    const logDir = join(dataDir, "events", String(workspaceId));
    const log = await EventLog.open(logDir);
    this.logs.set(workspaceId, log);
    this.statusMap.set(workspaceId, "idle");

    const session = new AgentSession(workspaceId, log, {
      onEvent: (event) => this.onEvent(event),
      onStatus: (status) => this.onStatus(status),
    });
    this.sessions.set(workspaceId, session);
  }

  private onEvent(event: AgentEvent): void {
    this.ring.push(event);
    if (this.ring.length > RING_CAP) {
      this.ring.shift();
    }

    if (this.connectedWs) {
      // TODO: chunking for large events (gotcha 10)
      this.connectedWs.send(encodeMsg(event));
    }
  }

  private onStatus(statusMsg: AgentStatus): void {
    this.statusMap.set(statusMsg.workspace_id, statusMsg.status);
    if (this.connectedWs) {
      this.connectedWs.send(encodeMsg(statusMsg));
    }
  }

  setClient(ws: WebSocket | null): void {
    this.connectedWs = ws;
  }

  getSession(id: number): AgentSession | undefined {
    return this.sessions.get(id);
  }

  getLog(id: number): EventLog | undefined {
    return this.logs.get(id);
  }

  getSnapshots(): WorkspaceSnapshot[] {
    return Array.from(this.sessions.keys()).map((id) => ({
      workspace_id: id,
      status: this.statusMap.get(id) ?? "idle",
    }));
  }

  getRingBuffer(): AgentEvent[] {
    return [...this.ring];
  }
}
