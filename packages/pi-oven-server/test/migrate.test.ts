import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import Database from "better-sqlite3";
import { migrate, MigrationError } from "../src/state/migrate.js";
import { openDb } from "../src/state/db.js";

let workdir: string;
let migrationsDir: string;
let dbPath: string;

beforeEach(() => {
  workdir = mkdtempSync(join(tmpdir(), "pi-oven-migrate-"));
  migrationsDir = join(workdir, "migrations");
  mkdirSync(migrationsDir, { recursive: true });
  dbPath = join(workdir, "state.db");
});

afterEach(() => {
  rmSync(workdir, { recursive: true, force: true });
});

function listBackups(): string[] {
  return readdirSync(workdir).filter((n) =>
    /^state\.db\.bak\.\d+$/.test(n),
  );
}

describe("migrate", () => {
  it("applies all migrations on a fresh database, taking a single backup", async () => {
    writeFileSync(
      join(migrationsDir, "0001_initial.sql"),
      `CREATE TABLE IF NOT EXISTS _migrations (
         id INTEGER PRIMARY KEY,
         name TEXT NOT NULL UNIQUE,
         checksum TEXT NOT NULL,
         applied_at INTEGER NOT NULL
       );`,
    );
    writeFileSync(
      join(migrationsDir, "0002_add_projects.sql"),
      "CREATE TABLE projects (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
    );

    const db = openDb(dbPath);
    try {
      const result = await migrate(db, migrationsDir);
      expect(result.applied).toEqual([
        "0001_initial.sql",
        "0002_add_projects.sql",
      ]);
      expect(result.backupPath).toBeDefined();
      expect(listBackups()).toHaveLength(1);

      const rows = db.prepare("SELECT name FROM _migrations ORDER BY name").all() as { name: string }[];
      expect(rows.map((r) => r.name)).toEqual([
        "0001_initial.sql",
        "0002_add_projects.sql",
      ]);
    } finally {
      db.close();
    }
  });

  it("is a no-op when the database is already current (no backup taken)", async () => {
    writeFileSync(
      join(migrationsDir, "0001_initial.sql"),
      `CREATE TABLE IF NOT EXISTS _migrations (
         id INTEGER PRIMARY KEY,
         name TEXT NOT NULL UNIQUE,
         checksum TEXT NOT NULL,
         applied_at INTEGER NOT NULL
       );`,
    );

    {
      const db = openDb(dbPath);
      try {
        await migrate(db, migrationsDir);
      } finally {
        db.close();
      }
    }

    const before = listBackups().length;
    const db = openDb(dbPath);
    try {
      const result = await migrate(db, migrationsDir);
      expect(result.applied).toEqual([]);
      expect(result.backupPath).toBeUndefined();
      expect(listBackups().length).toBe(before);
    } finally {
      db.close();
    }
  });

  it("only applies the remaining migrations when partially applied", async () => {
    writeFileSync(
      join(migrationsDir, "0001_initial.sql"),
      `CREATE TABLE IF NOT EXISTS _migrations (
         id INTEGER PRIMARY KEY,
         name TEXT NOT NULL UNIQUE,
         checksum TEXT NOT NULL,
         applied_at INTEGER NOT NULL
       );`,
    );

    {
      const db = openDb(dbPath);
      try {
        await migrate(db, migrationsDir);
      } finally {
        db.close();
      }
    }

    writeFileSync(
      join(migrationsDir, "0002_add_projects.sql"),
      "CREATE TABLE projects (id INTEGER PRIMARY KEY);",
    );

    const db = openDb(dbPath);
    try {
      const result = await migrate(db, migrationsDir);
      expect(result.applied).toEqual(["0002_add_projects.sql"]);
      expect(result.backupPath).toBeDefined();
    } finally {
      db.close();
    }
  });

  it("refuses to start and leaves the DB unchanged when an applied migration's checksum is tampered", async () => {
    const initialPath = join(migrationsDir, "0001_initial.sql");
    writeFileSync(
      initialPath,
      `CREATE TABLE IF NOT EXISTS _migrations (
         id INTEGER PRIMARY KEY,
         name TEXT NOT NULL UNIQUE,
         checksum TEXT NOT NULL,
         applied_at INTEGER NOT NULL
       );`,
    );

    {
      const db = openDb(dbPath);
      try {
        await migrate(db, migrationsDir);
      } finally {
        db.close();
      }
    }

    // Tamper: rewrite the file with semantically different content
    writeFileSync(
      initialPath,
      `CREATE TABLE IF NOT EXISTS _migrations (
         id INTEGER PRIMARY KEY,
         name TEXT NOT NULL UNIQUE,
         checksum TEXT NOT NULL,
         applied_at INTEGER NOT NULL,
         extra_col TEXT
       );`,
    );

    writeFileSync(
      join(migrationsDir, "0002_pending.sql"),
      "CREATE TABLE pending_marker (id INTEGER PRIMARY KEY);",
    );

    const db = openDb(dbPath);
    try {
      await expect(migrate(db, migrationsDir)).rejects.toThrow(MigrationError);
      // Pending should not have been applied
      const has = db
        .prepare(
          "SELECT name FROM sqlite_master WHERE type='table' AND name='pending_marker'",
        )
        .get();
      expect(has).toBeUndefined();

      const recorded = db.prepare("SELECT name FROM _migrations").all() as { name: string }[];
      expect(recorded.map((r) => r.name)).toEqual(["0001_initial.sql"]);
    } finally {
      db.close();
    }
  });

  it("rolls back a migration whose statements throw, leaving _migrations unchanged", async () => {
    writeFileSync(
      join(migrationsDir, "0001_initial.sql"),
      `CREATE TABLE IF NOT EXISTS _migrations (
         id INTEGER PRIMARY KEY,
         name TEXT NOT NULL UNIQUE,
         checksum TEXT NOT NULL,
         applied_at INTEGER NOT NULL
       );`,
    );
    // First statement is fine; second is invalid SQL — db.exec runs them as a script and should throw
    writeFileSync(
      join(migrationsDir, "0002_bad.sql"),
      `CREATE TABLE good (id INTEGER PRIMARY KEY);
       INVALID_SQL_THAT_DOES_NOT_PARSE;`,
    );

    const db = openDb(dbPath);
    try {
      await expect(migrate(db, migrationsDir)).rejects.toThrow();

      // _migrations should only contain 0001
      const recorded = db.prepare("SELECT name FROM _migrations").all() as { name: string }[];
      expect(recorded.map((r) => r.name)).toEqual(["0001_initial.sql"]);

      // good table from the failed migration must not exist
      const has = db
        .prepare(
          "SELECT name FROM sqlite_master WHERE type='table' AND name='good'",
        )
        .get();
      expect(has).toBeUndefined();
    } finally {
      db.close();
    }
  });
});
