import { createHash } from "node:crypto";
import {
  existsSync,
  readFileSync,
  readdirSync,
  unlinkSync,
} from "node:fs";
import { basename, dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import type { Database } from "better-sqlite3";

const FILE_RE = /^\d{4}_.*\.(sql|ts)$/;

export class MigrationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "MigrationError";
  }
}

export interface MigrateResult {
  applied: string[];
  backupPath?: string;
  prunedBackups: string[];
}

interface AppliedRow {
  name: string;
  checksum: string;
}

function sha256(buf: Buffer | string): string {
  return createHash("sha256").update(buf).digest("hex");
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function pruneBackups(dbPath: string, keep: number): string[] {
  const dir = dirname(dbPath);
  const base = basename(dbPath);
  const re = new RegExp(`^${escapeRegex(base)}\\.bak\\.(\\d+)$`);

  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return [];
  }
  const matches = entries
    .map((name) => {
      const m = name.match(re);
      return m ? { name, ts: Number(m[1]) } : null;
    })
    .filter((x): x is { name: string; ts: number } => x !== null)
    .sort((a, b) => b.ts - a.ts);

  const pruned: string[] = [];
  for (const m of matches.slice(keep)) {
    try {
      unlinkSync(join(dir, m.name));
      pruned.push(m.name);
    } catch {
      /* best-effort prune */
    }
  }
  return pruned;
}

export async function migrate(
  db: Database,
  migrationsDir: string,
): Promise<MigrateResult> {
  db.exec(`CREATE TABLE IF NOT EXISTS _migrations (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    checksum TEXT NOT NULL,
    applied_at INTEGER NOT NULL
  )`);

  const appliedRows = db
    .prepare("SELECT name, checksum FROM _migrations ORDER BY name")
    .all() as AppliedRow[];
  const appliedByName = new Map(appliedRows.map((r) => [r.name, r.checksum]));

  let files: string[];
  try {
    files = readdirSync(migrationsDir).filter((n) => FILE_RE.test(n));
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === "ENOENT") {
      files = [];
    } else {
      throw err;
    }
  }
  files.sort();

  // Verify checksums of applied migrations
  for (const [name, expected] of appliedByName) {
    const filePath = join(migrationsDir, name);
    if (!existsSync(filePath)) {
      throw new MigrationError(
        `Applied migration "${name}" is missing from ${migrationsDir}`,
      );
    }
    const actual = sha256(readFileSync(filePath));
    if (actual !== expected) {
      throw new MigrationError(
        `Checksum mismatch for "${name}" — recorded ${expected}, found ${actual}; refusing to start`,
      );
    }
  }

  const pending = files.filter((f) => !appliedByName.has(f));
  if (pending.length === 0) {
    return { applied: [], prunedBackups: [] };
  }

  const dbPath = (db as unknown as { name: string }).name;
  const backupPath = `${dbPath}.bak.${Date.now()}`;
  await db.backup(backupPath);

  const insertStmt = db.prepare(
    "INSERT INTO _migrations(name, checksum, applied_at) VALUES (?, ?, ?)",
  );

  const applied: string[] = [];
  for (const name of pending) {
    const filePath = join(migrationsDir, name);
    const bytes = readFileSync(filePath);
    const checksum = sha256(bytes);

    db.exec("BEGIN IMMEDIATE");
    try {
      if (name.endsWith(".sql")) {
        db.exec(bytes.toString("utf8"));
      } else {
        const mod = (await import(pathToFileURL(filePath).href)) as {
          up?: (db: Database) => void | Promise<void>;
        };
        if (typeof mod.up !== "function") {
          throw new MigrationError(
            `Migration "${name}" does not export an "up(db)" function`,
          );
        }
        await mod.up(db);
      }
      insertStmt.run(name, checksum, Date.now());
      db.exec("COMMIT");
      applied.push(name);
    } catch (err) {
      try {
        db.exec("ROLLBACK");
      } catch {
        /* ignore rollback failure */
      }
      throw err;
    }
  }

  const prunedBackups = pruneBackups(dbPath, 10);

  return { applied, backupPath, prunedBackups };
}
