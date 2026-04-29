export interface Hello {
  type: "Hello";
  key: string;
  client_version: string;
}

export interface Welcome {
  type: "Welcome";
  server_version: string;
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

export type Msg = Hello | Welcome | AuthFailed | Ping | Pong;

const KNOWN_TYPES = new Set<string>(["Hello", "Welcome", "AuthFailed", "Ping", "Pong"]);

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
