import { chmodSync, existsSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";
import Database, { type Database as DatabaseType } from "better-sqlite3";

export interface OpenDbOptions {
  /** When true, do not chmod a freshly created file. Used by tests on platforms with quirky chmod. */
  skipChmod?: boolean;
}

/**
 * Opens (or creates) the SQLite database at `path`. New files are created with mode 0600.
 * The required pragmas are applied in the documented order before this function returns,
 * so callers can never use this database without them.
 */
export function openDb(path: string, opts: OpenDbOptions = {}): DatabaseType {
  mkdirSync(dirname(path), { recursive: true });
  const fresh = !existsSync(path);

  const db = new Database(path);

  if (fresh && !opts.skipChmod) {
    try {
      chmodSync(path, 0o600);
    } catch {
      /* best-effort; tests on some FS may not honor chmod */
    }
  }

  db.pragma("journal_mode = WAL");
  db.pragma("synchronous = NORMAL");
  db.pragma("foreign_keys = ON");
  db.pragma("busy_timeout = 5000");
  db.pragma("temp_store = MEMORY");

  return db;
}
