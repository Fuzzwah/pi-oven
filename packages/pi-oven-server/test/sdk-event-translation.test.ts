import { describe, it, expect, afterEach } from "vitest";
import type { AgentSessionEvent } from "@mariozechner/pi-coding-agent";
import { translateSdkEvent, applyChildProcessEnv } from "../src/workspaces/session.js";

// Cast helper: the SDK's AgentSessionEvent union is huge and includes nested types
// from pi-ai/AssistantMessage; for these table-driven tests we only care about the
// subset of fields the translator inspects, so we hand-build minimal objects.
const ev = (x: unknown): AgentSessionEvent => x as AgentSessionEvent;

describe("translateSdkEvent", () => {
  it("agent_start → status running", () => {
    expect(translateSdkEvent(ev({ type: "agent_start" }))).toEqual({
      kind: "status",
      status: "running",
    });
  });

  it("agent_end → status idle", () => {
    expect(translateSdkEvent(ev({ type: "agent_end", messages: [] }))).toEqual({
      kind: "status",
      status: "idle",
    });
  });

  it("message_update with inner text_delta → text_delta event with renamed field", () => {
    const result = translateSdkEvent(
      ev({
        type: "message_update",
        message: {},
        assistantMessageEvent: {
          type: "text_delta",
          contentIndex: 0,
          delta: "hello",
          partial: {},
        },
      }),
    );
    expect(result).toEqual({
      kind: "event",
      event: { type: "text_delta", text: "hello" },
    });
  });

  it("message_update with non-text_delta inner event drops", () => {
    const result = translateSdkEvent(
      ev({
        type: "message_update",
        message: {},
        assistantMessageEvent: {
          type: "thinking_delta",
          contentIndex: 0,
          delta: "...",
          partial: {},
        },
      }),
    );
    expect(result).toEqual({ kind: "drop" });
  });

  it("tool_execution_start → tool_call event", () => {
    const result = translateSdkEvent(
      ev({
        type: "tool_execution_start",
        toolCallId: "t1",
        toolName: "bash",
        args: { command: "ls" },
      }),
    );
    expect(result).toEqual({
      kind: "event",
      event: { type: "tool_call", tool_name: "bash", args: { command: "ls" } },
    });
  });

  it("tool_execution_end success → tool_result with exit_code 0, string result passthrough", () => {
    const result = translateSdkEvent(
      ev({
        type: "tool_execution_end",
        toolCallId: "t1",
        toolName: "bash",
        result: "file1\nfile2",
        isError: false,
      }),
    );
    expect(result).toEqual({
      kind: "event",
      event: {
        type: "tool_result",
        tool_name: "bash",
        output: "file1\nfile2",
        exit_code: 0,
      },
    });
  });

  it("tool_execution_end failure → tool_result with exit_code 1", () => {
    const result = translateSdkEvent(
      ev({
        type: "tool_execution_end",
        toolCallId: "t1",
        toolName: "bash",
        result: "permission denied",
        isError: true,
      }),
    );
    expect(result).toEqual({
      kind: "event",
      event: {
        type: "tool_result",
        tool_name: "bash",
        output: "permission denied",
        exit_code: 1,
      },
    });
  });

  it("tool_execution_end with non-string result is JSON-stringified", () => {
    const result = translateSdkEvent(
      ev({
        type: "tool_execution_end",
        toolCallId: "t1",
        toolName: "read",
        result: { lines: ["a", "b"], count: 2 },
        isError: false,
      }),
    );
    expect(result).toEqual({
      kind: "event",
      event: {
        type: "tool_result",
        tool_name: "read",
        output: '{"lines":["a","b"],"count":2}',
        exit_code: 0,
      },
    });
  });

  for (const droppedType of [
    "turn_start",
    "turn_end",
    "message_start",
    "message_end",
    "tool_execution_update",
    "queue_update",
    "compaction_start",
    "compaction_end",
    "auto_retry_start",
    "auto_retry_end",
    "session_info_changed",
  ]) {
    it(`${droppedType} drops`, () => {
      expect(translateSdkEvent(ev({ type: droppedType }))).toEqual({ kind: "drop" });
    });
  }

  it("unknown event type drops (forward-compatibility)", () => {
    expect(translateSdkEvent(ev({ type: "some_future_event_type" }))).toEqual({
      kind: "drop",
    });
  });
});

describe("applyChildProcessEnv", () => {
  const saved: Record<string, string | undefined> = {};
  const keys = [
    "LANG",
    "TZ",
    "EDITOR",
    "TERM",
    "NO_COLOR",
    "GIT_TERMINAL_PROMPT",
    "PI_OVEN_WORKSPACE_ID",
  ];

  afterEach(() => {
    for (const k of keys) {
      if (saved[k] === undefined) delete process.env[k];
      else process.env[k] = saved[k];
      delete saved[k];
    }
  });

  function snapshot(): void {
    for (const k of keys) {
      saved[k] = process.env[k];
      delete process.env[k];
    }
  }

  it("sets all defaults when env is empty", () => {
    snapshot();
    applyChildProcessEnv(7);
    expect(process.env.LANG).toBe("en_US.UTF-8");
    expect(process.env.TZ).toBe("UTC");
    expect(process.env.EDITOR).toBe("true");
    expect(process.env.TERM).toBe("dumb");
    expect(process.env.NO_COLOR).toBe("1");
    expect(process.env.GIT_TERMINAL_PROMPT).toBe("0");
    expect(process.env.PI_OVEN_WORKSPACE_ID).toBe("7");
  });

  it("preserves user-set LANG and applies the rest", () => {
    snapshot();
    process.env.LANG = "de_DE.UTF-8";
    applyChildProcessEnv(1);
    expect(process.env.LANG).toBe("de_DE.UTF-8");
    expect(process.env.EDITOR).toBe("true");
    expect(process.env.TZ).toBe("UTC");
    expect(process.env.PI_OVEN_WORKSPACE_ID).toBe("1");
  });

  it("does not overwrite PI_OVEN_WORKSPACE_ID if already set", () => {
    snapshot();
    process.env.PI_OVEN_WORKSPACE_ID = "999";
    applyChildProcessEnv(1);
    expect(process.env.PI_OVEN_WORKSPACE_ID).toBe("999");
  });
});
