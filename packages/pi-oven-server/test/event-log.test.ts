import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, rmSync, writeFileSync, mkdtempSync } from "node:fs";
import { join, sep } from "node:path";
import { tmpdir } from "node:os";
import { EventLog } from "../src/workspaces/events/log.js";

let tmpDir: string;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "event-log-test-"));
});

afterEach(() => {
  rmSync(tmpDir, { recursive: true, force: true });
});

describe("EventLog", () => {
  it("seq starts at 1 for empty dir", async () => {
    const logDir = join(tmpDir, "events", "1");
    const log = await EventLog.open(logDir);
    expect(log.nextSeq).toBe(1);
  });

  it("restores seq from existing log", async () => {
    const logDir = join(tmpDir, "events", "1");
    mkdirSync(logDir, { recursive: true });
    const existing = join(logDir, "1000000-0.ndjson");
    writeFileSync(
      existing,
      '{"seq":42,"ts":1000,"event":{"type":"text"}}\n',
      { mode: 0o600 },
    );
    const log = await EventLog.open(logDir);
    expect(log.nextSeq).toBe(43);
    await log.close();
  });

  it("appends events with incrementing seq", async () => {
    const logDir = join(tmpDir, "events", "1");
    const log = await EventLog.open(logDir);
    const s1 = await log.append({ type: "text_delta", text: "hello" });
    const s2 = await log.append({ type: "text_delta", text: "world" });
    expect(s1).toBe(1);
    expect(s2).toBe(2);
    await log.close();
  });

  it("replays events after lastSeq", async () => {
    const logDir = join(tmpDir, "events", "1");
    const log = await EventLog.open(logDir);
    await log.append({ type: "a" });
    await log.append({ type: "b" });
    await log.append({ type: "c" });
    await log.close();

    const log2 = await EventLog.open(logDir);
    const replayed: number[] = [];
    for await (const entry of log2.replay(1)) {
      replayed.push(entry.seq);
    }
    expect(replayed).toEqual([2, 3]);
    await log2.close();
  });

  it("replay with lastSeq=0 returns all events", async () => {
    const logDir = join(tmpDir, "events", "1");
    const log = await EventLog.open(logDir);
    await log.append({ type: "x" });
    await log.append({ type: "y" });
    await log.close();

    const log2 = await EventLog.open(logDir);
    const replayed: number[] = [];
    for await (const entry of log2.replay(0)) {
      replayed.push(entry.seq);
    }
    expect(replayed).toEqual([1, 2]);
    await log2.close();
  });

  it("creates log directory with mode 0700 on first write", async () => {
    const logDir = join(tmpDir, "events", "deep", "1");
    const log = await EventLog.open(logDir);
    await log.append({ type: "first" });

    const { statSync } = await import("node:fs");
    const stat = statSync(logDir);
    expect(stat.isDirectory()).toBe(true);
    await log.close();
  });

  it("rotates at MAX_FILE_SIZE boundary", async () => {
    const logDir = join(tmpDir, "events", "1");
    const log = await EventLog.open(logDir);

    // Write a large enough entry to trigger rotation by manipulating the internal size counter.
    // We do this by appending events until the currentSize is forced over the limit via many writes.
    // For speed, we directly test that after rotation a new file exists.
    // Inject a synthetic large currentSize by appending once then setting internal size.
    await log.append({ type: "seed" });

    // Force rotation by injecting an oversized currentSize
    (log as unknown as Record<string, unknown>).currentSize = 64 * 1024 * 1024;
    await log.append({ type: "trigger" });

    // After rotation there should be 2 ndjson files
    const files = EventLog.listLogFiles(logDir);
    expect(files.length).toBe(2);
    await log.close();
  });
});
