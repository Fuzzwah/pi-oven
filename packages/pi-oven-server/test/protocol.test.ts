import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, it, expect } from "vitest";
import {
  decodeMsg,
  encodeMsg,
  isHello,
  isWelcome,
  isAuthFailed,
  isPing,
  isPong,
} from "../src/protocol.js";
import type { Msg } from "../src/protocol.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

// Same fixture bytes used in crates/pi-oven-protocol/tests/msg.rs.
const HELLO_FIXTURE = `{"type":"Hello","key":"fixture-key","client_version":"0.1.0"}`;

describe("encodeMsg / decodeMsg round-trip", () => {
  it("Hello", () => {
    const msg: Msg = { type: "Hello", key: "k", client_version: "0.1.0" };
    const rt = decodeMsg(encodeMsg(msg));
    expect(rt).toEqual(msg);
  });

  it("Welcome", () => {
    const msg: Msg = { type: "Welcome", server_version: "1.2.3", workspaces: [] };
    expect(decodeMsg(encodeMsg(msg))).toEqual(msg);
  });

  it("AuthFailed", () => {
    const msg: Msg = { type: "AuthFailed", reason: "invalid_key" };
    expect(decodeMsg(encodeMsg(msg))).toEqual(msg);
  });

  it("Ping", () => {
    const msg: Msg = { type: "Ping", ts_ms: 12345 };
    expect(decodeMsg(encodeMsg(msg))).toEqual(msg);
  });

  it("Pong", () => {
    const msg: Msg = { type: "Pong", client_ts_ms: 100, server_ts_ms: 200 };
    expect(decodeMsg(encodeMsg(msg))).toEqual(msg);
  });
});

describe("decodeMsg rejects bad input", () => {
  it("unknown type returns null", () => {
    expect(decodeMsg(`{"type":"DefinitelyNotAMessage"}`)).toBeNull();
  });

  it("malformed JSON returns null", () => {
    expect(decodeMsg("not json")).toBeNull();
  });

  it("missing type field returns null", () => {
    expect(decodeMsg(`{"key":"k"}`)).toBeNull();
  });

  it("null input object returns null", () => {
    expect(decodeMsg("null")).toBeNull();
  });
});

describe("type guards", () => {
  it("isHello", () => {
    const m = decodeMsg(encodeMsg({ type: "Hello", key: "k", client_version: "0.0.0" }))!;
    expect(isHello(m)).toBe(true);
    expect(isWelcome(m)).toBe(false);
  });

  it("isPing", () => {
    const m = decodeMsg(encodeMsg({ type: "Ping", ts_ms: 1 }))!;
    expect(isPing(m)).toBe(true);
    expect(isPong(m)).toBe(false);
  });

  it("isAuthFailed", () => {
    const m = decodeMsg(encodeMsg({ type: "AuthFailed", reason: "r" }))!;
    expect(isAuthFailed(m)).toBe(true);
  });
});

describe("cross-language fixture (task 2.6)", () => {
  it("decodes the shared Hello fixture from Rust tests", () => {
    const msg = decodeMsg(HELLO_FIXTURE);
    expect(msg).not.toBeNull();
    expect(msg!.type).toBe("Hello");
    if (isHello(msg!)) {
      expect(msg.key).toBe("fixture-key");
      expect(msg.client_version).toBe("0.1.0");
    }
  });

  it("re-encoding and re-decoding is stable", () => {
    const msg = decodeMsg(HELLO_FIXTURE)!;
    const msg2 = decodeMsg(encodeMsg(msg));
    expect(msg2).toEqual(msg);
  });
});

describe("new message variant fixture round-trips (task 1.5)", () => {
  const fixturesDir = join(__dirname, "fixtures", "protocol");
  const variants = [
    "Send",
    "Abort",
    "AgentEvent",
    "AgentStatus",
    "Resume",
    "ReplayBatch",
    "ErrorEvent",
    "Welcome",
  ];

  for (const name of variants) {
    it(`${name} round-trips`, () => {
      const raw = readFileSync(join(fixturesDir, `${name}.json`), "utf8").trim();
      const msg = decodeMsg(raw);
      expect(msg, `${name}.json failed to decode`).not.toBeNull();
      const reencoded = encodeMsg(msg!);
      const rt = decodeMsg(reencoded);
      expect(rt).toEqual(msg);
    });
  }
});
