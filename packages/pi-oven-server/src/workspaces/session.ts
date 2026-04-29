import type {
  AgentSession as PiAgentSession,
  AgentSessionEvent,
} from "@mariozechner/pi-coding-agent";

import type { AgentEvent, AgentStatus } from "../protocol.js";
import { EventLog } from "./events/log.js";

type AgentStatusKind = "running" | "idle";

export interface AgentSessionCallbacks {
  onEvent: (event: AgentEvent) => void;
  onStatus: (status: AgentStatus) => void;
}

export type Translation =
  | { kind: "event"; event: unknown }
  | { kind: "status"; status: AgentStatusKind }
  | { kind: "drop" };

export function translateSdkEvent(ev: AgentSessionEvent): Translation {
  switch (ev.type) {
    case "agent_start":
      return { kind: "status", status: "running" };
    case "agent_end":
      return { kind: "status", status: "idle" };
    case "message_update": {
      const inner = ev.assistantMessageEvent;
      if (inner && inner.type === "text_delta") {
        return { kind: "event", event: { type: "text_delta", text: inner.delta } };
      }
      return { kind: "drop" };
    }
    case "tool_execution_start":
      return {
        kind: "event",
        event: { type: "tool_call", tool_name: ev.toolName, args: ev.args },
      };
    case "tool_execution_end":
      return {
        kind: "event",
        event: {
          type: "tool_result",
          tool_name: ev.toolName,
          // Stringify non-string results so the renderer (which reads `output` as string) renders sanely.
          output: typeof ev.result === "string" ? ev.result : JSON.stringify(ev.result),
          exit_code: ev.isError ? 1 : 0,
        },
      };
    default:
      return { kind: "drop" };
  }
}

// Apply gotcha-12 env defaults to process.env. pi runs in-process and its bash tool
// inherits process.env; this is best-effort isolation until a custom BashOperations lands.
// Only sets keys that are unset, to respect deliberately-set user values like LANG=de_DE.UTF-8.
export function applyChildProcessEnv(workspaceId: number): void {
  const defaults: Record<string, string> = {
    LANG: "en_US.UTF-8",
    TZ: "UTC",
    EDITOR: "true",
    TERM: "dumb",
    NO_COLOR: "1",
    GIT_TERMINAL_PROMPT: "0",
    PI_OVEN_WORKSPACE_ID: String(workspaceId),
  };
  for (const [k, v] of Object.entries(defaults)) {
    if (process.env[k] === undefined) {
      process.env[k] = v;
    }
  }
}

export class AgentSession {
  private readonly workspaceId: number;
  private readonly log: EventLog;
  private readonly cwd: string;
  private readonly callbacks: AgentSessionCallbacks;
  private status: AgentStatusKind = "idle";
  private stubTimers: ReturnType<typeof setTimeout>[] = [];
  private piSession?: PiAgentSession;
  private piUnsubscribe?: () => void;

  constructor(
    workspaceId: number,
    log: EventLog,
    cwd: string,
    callbacks: AgentSessionCallbacks,
  ) {
    this.workspaceId = workspaceId;
    this.log = log;
    this.cwd = cwd;
    this.callbacks = callbacks;
  }

  async init(): Promise<void> {
    if (process.env.PI_OVEN_SDK_STUB === "1") return;
    applyChildProcessEnv(this.workspaceId);
    // Dynamic import: the SDK pulls in a large module graph (provider registry,
    // timers, etc.) that we don't want loaded in stub-only or test paths.
    const { createAgentSession } = await import("@mariozechner/pi-coding-agent");
    const result = await createAgentSession({ cwd: this.cwd });
    if (!result.session.model) {
      throw new Error(
        `pi SDK has no usable model: ${result.modelFallbackMessage ?? "no auth configured for any registered model"}`,
      );
    }
    this.piSession = result.session;
    this.piUnsubscribe = this.piSession.subscribe((ev) => this.onPiEvent(ev));
  }

  async queue(text: string, mode: "steer" | "followup"): Promise<void> {
    if (process.env.PI_OVEN_SDK_STUB === "1") {
      await this.stubQueue(text, mode);
      return;
    }
    if (!this.piSession) {
      throw new Error("AgentSession.init() not called");
    }
    await this.piSession.prompt(text, {
      streamingBehavior: mode === "steer" ? "steer" : "followUp",
      source: "interactive",
    });
  }

  async abort(): Promise<void> {
    if (process.env.PI_OVEN_SDK_STUB === "1") {
      for (const t of this.stubTimers) clearTimeout(t);
      this.stubTimers = [];
      await this.emitStatus("idle");
      return;
    }
    await this.piSession?.abort();
    // Belt-and-braces: guarantee idle even if the SDK doesn't emit agent_end after abort.
    await this.emitStatus("idle");
  }

  async dispose(): Promise<void> {
    this.piUnsubscribe?.();
    this.piUnsubscribe = undefined;
    this.piSession?.dispose();
    this.piSession = undefined;
  }

  getStatus(): AgentStatusKind {
    return this.status;
  }

  private onPiEvent(ev: AgentSessionEvent): void {
    const t = translateSdkEvent(ev);
    if (t.kind === "event") {
      void this.emitEvent(t.event);
    } else if (t.kind === "status") {
      void this.emitStatus(t.status);
    }
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
