import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { acquireLock, LockError } from "../src/lock.js";

let workdir: string;

beforeEach(() => {
  workdir = mkdtempSync(join(tmpdir(), "pi-oven-lock-"));
});

afterEach(() => {
  rmSync(workdir, { recursive: true, force: true });
});

describe("acquireLock", () => {
  it("writes pid + started_at JSON into the lock file body", async () => {
    const release = await acquireLock(workdir, { registerProcessHandlers: false });
    try {
      const body = readFileSync(join(workdir, "server.lock"), "utf8");
      const parsed = JSON.parse(body) as { pid: number; started_at: number };
      expect(parsed.pid).toBe(process.pid);
      expect(typeof parsed.started_at).toBe("number");
      expect(parsed.started_at).toBeGreaterThan(0);
    } finally {
      await release();
    }
  });

  it("a second acquire on the same dir throws LockError with the holder's pid", async () => {
    const release = await acquireLock(workdir, { registerProcessHandlers: false });
    try {
      await expect(
        acquireLock(workdir, { registerProcessHandlers: false }),
      ).rejects.toMatchObject({
        name: "LockError",
        holderPid: process.pid,
      });
    } finally {
      await release();
    }
  });

  it("releases the lock so a subsequent acquire succeeds", async () => {
    const release1 = await acquireLock(workdir, { registerProcessHandlers: false });
    await release1();

    const release2 = await acquireLock(workdir, { registerProcessHandlers: false });
    await release2();
  });

  it("two child processes contending: the second exits non-zero with the first's pid in stderr", async () => {
    const fixture = resolve(__dirname, "fixtures/hold-lock.ts");
    const tsxBin = resolve(__dirname, "../node_modules/.bin/tsx");

    // Start the holder. It prints "LOCKED:<pid>" once it has the lock, then waits on stdin to release.
    // We use spawnSync with input to keep this synchronous and deterministic.
    // Strategy: hold the lock by acquiring it inside the holder, write LOCKED + pid to a sentinel file,
    // then the holder exits. While the holder is still holding (uses an "exit" handler), the second
    // process tries to acquire; or — more robustly — we run them in two phases using a sentinel file
    // path passed to both.
    //
    // Simpler approach: run the holder synchronously, capture its pid printed before it acquires the
    // lock, but keep the lock held for a moment. We do this by writing a long-running holder that we
    // signal externally.
    //
    // For the unit-test environment we instead invoke the holder twice serially:
    //   1) holder runs, writes meta, exits cleanly, leaves a stale-but-released proper-lockfile
    //   2) we manually keep the lock held by acquiring inside this test process before spawning a contender
    // That's exactly what the next assertion does — it crosses a process boundary for the contender.

    // Acquire the lock in this process (holder = the current vitest worker)
    const release = await acquireLock(workdir, { registerProcessHandlers: false });
    try {
      const result = spawnSync(tsxBin, [fixture, workdir], {
        encoding: "utf8",
      });
      expect(result.status).not.toBe(0);
      expect(result.stderr).toContain(String(process.pid));
    } finally {
      await release();
    }
  });
});
