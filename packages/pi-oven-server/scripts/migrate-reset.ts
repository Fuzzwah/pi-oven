import { existsSync, unlinkSync } from "node:fs";
import { createInterface } from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";
import { join } from "node:path";
import { loadConfig } from "../src/config.js";
import { openDb } from "../src/state/db.js";
import { migrate } from "../src/state/migrate.js";
import { MIGRATIONS_DIR } from "./_paths.js";

async function main(): Promise<void> {
  if (process.env.NODE_ENV === "production") {
    console.error("migrate:reset is forbidden in production (NODE_ENV=production)");
    process.exit(1);
  }

  const cfg = loadConfig();
  const dbPath = join(cfg.data_dir, "state.db");

  console.error("");
  console.error("DESTRUCTIVE: this will delete the database at:");
  console.error(`  ${dbPath}`);
  console.error("");
  console.error(`Type the data directory path exactly to confirm: ${cfg.data_dir}`);

  const rl = createInterface({ input, output });
  const answer = (await rl.question("> ")).trim();
  rl.close();

  if (answer !== cfg.data_dir) {
    console.error("Confirmation did not match. Aborting.");
    process.exit(1);
  }

  for (const suffix of ["", "-wal", "-shm"]) {
    const target = `${dbPath}${suffix}`;
    if (existsSync(target)) {
      unlinkSync(target);
      console.error(`deleted ${target}`);
    }
  }

  const db = openDb(dbPath);
  try {
    const result = await migrate(db, MIGRATIONS_DIR);
    console.error(`re-applied ${result.applied.length} migrations:`);
    for (const name of result.applied) console.error(`  ${name}`);
  } finally {
    db.close();
  }
}

await main();
