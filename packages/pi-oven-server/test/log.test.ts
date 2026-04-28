import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  closeSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { initLogger, pruneOldLogs } from "../src/log.js";

let workdir: string;

beforeEach(() => {
  workdir = mkdtempSync(join(tmpdir(), "pi-oven-log-"));
});

afterEach(() => {
  rmSync(workdir, { recursive: true, force: true });
});

async function flushAndRead(filePath: string): Promise<string> {
  // Wait briefly for pino's async destination to flush
  for (let i = 0; i < 20; i++) {
    try {
      const text = readFileSync(filePath, "utf8");
      if (text.length > 0) return text;
    } catch {
      /* not yet */
    }
    await new Promise((r) => setTimeout(r, 50));
  }
  return readFileSync(filePath, "utf8");
}

describe("initLogger", () => {
  it("emits valid JSON with level, time, pid, msg fields", async () => {
    const { logger, logFilePath } = initLogger({
      data_dir: workdir,
      log_level: "info",
      tz: "UTC",
      development: false,
    });

    logger.info({ foo: "bar" }, "hello world");
    // Pino's async destination flushes on logger.flush() or process exit; force a flush
    (logger as unknown as { flush: () => void }).flush();

    const contents = await flushAndRead(logFilePath);
    const lines = contents.trim().split("\n").filter(Boolean);
    expect(lines.length).toBeGreaterThan(0);

    const parsed = JSON.parse(lines[lines.length - 1]!);
    expect(typeof parsed.level).toBe("number");
    expect(typeof parsed.time).toBe("number");
    expect(parsed.pid).toBe(process.pid);
    expect(parsed.msg).toBe("hello world");
    expect(parsed.foo).toBe("bar");
  });

  it("creates the log file in the date stamped name (configured TZ)", () => {
    const fixed = new Date("2025-06-15T15:30:00Z");
    const { logFilePath } = initLogger({
      data_dir: workdir,
      log_level: "info",
      tz: "Australia/Brisbane", // UTC+10 — 15:30 UTC = 01:30 next day
      development: false,
      now: fixed,
    });
    expect(logFilePath.endsWith("server-2025-06-16.ndjson")).toBe(true);
  });
});

describe("pruneOldLogs", () => {
  it("keeps the 7 most recent and deletes the rest", () => {
    const logsDir = join(workdir, "logs");
    mkdirSync(logsDir, { recursive: true });

    const dates = [
      "2025-01-01",
      "2025-01-02",
      "2025-01-03",
      "2025-01-04",
      "2025-01-05",
      "2025-01-06",
      "2025-01-07",
      "2025-01-08",
      "2025-01-09",
      "2025-01-10",
    ];
    for (const d of dates) {
      writeFileSync(join(logsDir, `server-${d}.ndjson`), "x");
    }
    // Add a non-matching file that should be left alone
    writeFileSync(join(logsDir, "ignore.txt"), "x");

    const pruned = pruneOldLogs(logsDir, 7);
    expect(pruned.sort()).toEqual([
      "server-2025-01-01.ndjson",
      "server-2025-01-02.ndjson",
      "server-2025-01-03.ndjson",
    ].sort());

    const remaining = readdirSync(logsDir).filter((n) => n.startsWith("server-"));
    expect(remaining).toHaveLength(7);
    expect(readdirSync(logsDir)).toContain("ignore.txt");
  });
});
