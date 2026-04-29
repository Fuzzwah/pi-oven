export interface WorkspaceSnapshot {
  workspace_id: number;
  status: "running" | "idle";
}

export interface StoredEvent {
  seq: number;
  ts: number;
  event: unknown;
}

export interface Hello {
  type: "Hello";
  key: string;
  client_version: string;
}

export interface Welcome {
  type: "Welcome";
  server_version: string;
  workspaces: WorkspaceSnapshot[];
}

export interface AuthFailed {
  type: "AuthFailed";
  reason: string;
}

export interface Ping {
  type: "Ping";
  ts_ms: number;
}

export interface Pong {
  type: "Pong";
  client_ts_ms: number;
  server_ts_ms: number;
}

export interface Send {
  type: "Send";
  workspace_id: number;
  text: string;
  queue_mode: "steer" | "followup";
}

export interface Abort {
  type: "Abort";
  workspace_id: number;
}

export interface AgentEvent {
  type: "AgentEvent";
  workspace_id: number;
  seq: number;
  event: unknown;
}

export interface AgentStatus {
  type: "AgentStatus";
  workspace_id: number;
  status: "running" | "idle";
}

export interface Resume {
  type: "Resume";
  workspace_id: number;
  last_seq: number;
}

export interface ReplayBatch {
  type: "ReplayBatch";
  workspace_id: number;
  events: StoredEvent[];
  latest_seq: number;
}

export interface ErrorEvent {
  type: "ErrorEvent";
  workspace_id?: number;
  reason: string;
}

export type Msg =
  | Hello
  | Welcome
  | AuthFailed
  | Ping
  | Pong
  | Send
  | Abort
  | AgentEvent
  | AgentStatus
  | Resume
  | ReplayBatch
  | ErrorEvent;

const KNOWN_TYPES = new Set<string>([
  "Hello",
  "Welcome",
  "AuthFailed",
  "Ping",
  "Pong",
  "Send",
  "Abort",
  "AgentEvent",
  "AgentStatus",
  "Resume",
  "ReplayBatch",
  "ErrorEvent",
]);

export function isHello(m: Msg): m is Hello {
  return m.type === "Hello";
}

export function isWelcome(m: Msg): m is Welcome {
  return m.type === "Welcome";
}

export function isAuthFailed(m: Msg): m is AuthFailed {
  return m.type === "AuthFailed";
}

export function isPing(m: Msg): m is Ping {
  return m.type === "Ping";
}

export function isPong(m: Msg): m is Pong {
  return m.type === "Pong";
}

export function isSend(m: Msg): m is Send {
  return m.type === "Send";
}

export function isAbort(m: Msg): m is Abort {
  return m.type === "Abort";
}

export function isAgentEvent(m: Msg): m is AgentEvent {
  return m.type === "AgentEvent";
}

export function isAgentStatus(m: Msg): m is AgentStatus {
  return m.type === "AgentStatus";
}

export function isResume(m: Msg): m is Resume {
  return m.type === "Resume";
}

export function isReplayBatch(m: Msg): m is ReplayBatch {
  return m.type === "ReplayBatch";
}

export function isErrorEvent(m: Msg): m is ErrorEvent {
  return m.type === "ErrorEvent";
}

export function decodeMsg(raw: string): Msg | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("type" in parsed) ||
    typeof (parsed as Record<string, unknown>).type !== "string"
  ) {
    return null;
  }
  const type = (parsed as Record<string, unknown>).type as string;
  if (!KNOWN_TYPES.has(type)) {
    return null;
  }
  return parsed as Msg;
}

export function encodeMsg(msg: Msg): string {
  return JSON.stringify(msg);
}
