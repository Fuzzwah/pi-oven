import { join } from "node:path";
import { existsSync, readdirSync } from "node:fs";
import { loadConfig } from "../src/config.js";
import { openDb } from "../src/state/db.js";
import { MIGRATIONS_DIR } from "./_paths.js";

const FILE_RE = /^\d{4}_.*\.(sql|ts)$/;

async function main(): Promise<void> {
  const cfg = loadConfig();
  const dbPath = join(cfg.data_dir, "state.db");

  let appliedRows: { name: string; applied_at: number }[] = [];
  if (existsSync(dbPath)) {
    const db = openDb(dbPath);
    try {
      const exists = db
        .prepare(
          "SELECT name FROM sqlite_master WHERE type='table' AND name='_migrations'",
        )
        .get();
      if (exists) {
        appliedRows = db
          .prepare(
            "SELECT name, applied_at FROM _migrations ORDER BY name",
          )
          .all() as { name: string; applied_at: number }[];
      }
    } finally {
      db.close();
    }
  }

  const appliedSet = new Set(appliedRows.map((r) => r.name));

  let allFiles: string[] = [];
  try {
    allFiles = readdirSync(MIGRATIONS_DIR).filter((n) => FILE_RE.test(n));
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err;
  }
  allFiles.sort();
  const pending = allFiles.filter((f) => !appliedSet.has(f));

  console.log(`Data dir:       ${cfg.data_dir}`);
  console.log(`Migrations dir: ${MIGRATIONS_DIR}`);
  console.log("");

  console.log(`Applied (${appliedRows.length}):`);
  if (appliedRows.length === 0) {
    console.log("  (none)");
  } else {
    for (const r of appliedRows) {
      const ts = new Date(r.applied_at).toISOString();
      console.log(`  ${r.name}  applied_at=${ts}`);
    }
  }
  console.log("");

  console.log(`Pending (${pending.length}):`);
  if (pending.length === 0) {
    console.log("  (none)");
  } else {
    for (const name of pending) {
      console.log(`  ${name}`);
    }
  }
}

await main();
