import type { AgentEvent, AgentStatus } from "../protocol.js";
import { EventLog } from "./events/log.js";

type AgentStatusKind = "running" | "idle";

export interface AgentSessionCallbacks {
  onEvent: (event: AgentEvent) => void;
  onStatus: (status: AgentStatus) => void;
}

// Environment variables passed to the pi SDK subprocess (gotcha 12).
function buildSpawnEnv(workspaceId: number): Record<string, string> {
  return {
    PATH: process.env.PATH ?? "/usr/local/bin:/usr/bin:/bin",
    LANG: process.env.LANG ?? "en_US.UTF-8",
    TZ: process.env.TZ ?? "UTC",
    EDITOR: "true",
    TERM: "dumb",
    NO_COLOR: "1",
    GIT_TERMINAL_PROMPT: "0",
    PI_OVEN_WORKSPACE_ID: String(workspaceId),
  };
}

export class AgentSession {
  private readonly workspaceId: number;
  private readonly log: EventLog;
  private readonly callbacks: AgentSessionCallbacks;
  private status: AgentStatusKind = "idle";
  private stubTimers: ReturnType<typeof setTimeout>[] = [];

  constructor(
    workspaceId: number,
    log: EventLog,
    callbacks: AgentSessionCallbacks,
  ) {
    this.workspaceId = workspaceId;
    this.log = log;
    this.callbacks = callbacks;
  }

  async queue(text: string, mode: "steer" | "followup"): Promise<void> {
    if (process.env.PI_OVEN_SDK_STUB === "1") {
      await this.stubQueue(text, mode);
      return;
    }
    // TODO: wire real @mariozechner/pi-coding-agent SDK
    // const { createAgentSession } = await import("@mariozechner/pi-coding-agent");
    // const _env = buildSpawnEnv(this.workspaceId);
    throw new Error("PI_OVEN_SDK_STUB not set and real SDK not wired");
  }

  async abort(): Promise<void> {
    if (process.env.PI_OVEN_SDK_STUB === "1") {
      for (const t of this.stubTimers) clearTimeout(t);
      this.stubTimers = [];
    }
    await this.emitStatus("idle");
  }

  getStatus(): AgentStatusKind {
    return this.status;
  }

  private async stubQueue(text: string, mode: string): Promise<void> {
    await this.emitStatus("running");

    const events: unknown[] = [
      { type: "text_delta", text: `[stub] you said: "${text}" (mode: ${mode})\n` },
      { type: "text_delta", text: "This is a synthetic response from the stub.\n" },
      { type: "tool_call", tool_name: "bash", args: { command: "echo hello" } },
      { type: "tool_result", tool_name: "bash", output: "hello\n", exit_code: 0 },
      { type: "text_delta", text: "Done.\n" },
    ];

    let idx = 0;
    const emitNext = async () => {
      if (idx >= events.length) {
        await this.emitStatus("idle");
        return;
      }
      await this.emitEvent(events[idx++]);
      const t = setTimeout(emitNext, 200);
      this.stubTimers.push(t);
    };

    const t = setTimeout(emitNext, 200);
    this.stubTimers.push(t);
  }

  private async emitEvent(piEvent: unknown): Promise<void> {
    const seq = await this.log.append(piEvent);
    const msg: AgentEvent = {
      type: "AgentEvent",
      workspace_id: this.workspaceId,
      seq,
      event: piEvent,
    };
    this.callbacks.onEvent(msg);
  }

  private async emitStatus(status: AgentStatusKind): Promise<void> {
    this.status = status;
    const msg: AgentStatus = {
      type: "AgentStatus",
      workspace_id: this.workspaceId,
      status,
    };
    this.callbacks.onStatus(msg);
  }
}
