import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mkdtempSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { openDb } from "../src/state/db.js";

let workdir: string;

beforeEach(() => {
  workdir = mkdtempSync(join(tmpdir(), "pi-oven-db-"));
});

afterEach(() => {
  rmSync(workdir, { recursive: true, force: true });
});

describe("openDb", () => {
  it("creates the file with mode 0600 and applies required pragmas", () => {
    const dbPath = join(workdir, "state.db");
    const db = openDb(dbPath);
    try {
      const stat = statSync(dbPath);
      expect((stat.mode & 0o777).toString(8)).toBe("600");

      expect(db.pragma("journal_mode", { simple: true })).toBe("wal");
      expect(db.pragma("synchronous", { simple: true })).toBe(1); // NORMAL
      expect(db.pragma("foreign_keys", { simple: true })).toBe(1);
      expect(db.pragma("busy_timeout", { simple: true })).toBe(5000);
      expect(db.pragma("temp_store", { simple: true })).toBe(2); // MEMORY
    } finally {
      db.close();
    }
  });
});
