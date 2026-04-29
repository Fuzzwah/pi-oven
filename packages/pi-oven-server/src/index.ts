import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import type { Logger } from "pino";
import type { Database } from "better-sqlite3";
import { loadConfig, type ServerConfig } from "./config.js";
import { acquireLock, type ReleaseLock } from "./lock.js";
import { initLogger } from "./log.js";
import { openDb } from "./state/db.js";
import { migrate } from "./state/migrate.js";

const VERSION = "0.0.0";

const __dirname = dirname(fileURLToPath(import.meta.url));
const MIGRATIONS_DIR = join(__dirname, "state", "migrations");

let logger: Logger | undefined;
let db: Database | undefined;
let release: ReleaseLock | undefined;

async function shutdown(signal: NodeJS.Signals): Promise<void> {
  if (logger) {
    logger.info({ signal }, "shutting down");
    try {
      (logger as unknown as { flush: () => void }).flush();
    } catch { /* ignore */ }
  }
  try {
    db?.close();
  } catch { /* ignore */ }
  try {
    await release?.();
  } catch { /* ignore */ }
  process.exit(0);
}

async function boot(): Promise<void> {
  let cfg: ServerConfig;
  let step = "load_config";

  try {
    cfg = loadConfig();
    step = "acquire_lock";
    release = await acquireLock(cfg.data_dir, { registerProcessHandlers: false });

    step = "init_logger";
    const init = initLogger({
      data_dir: cfg.data_dir,
      log_level: cfg.log_level,
      tz: cfg.tz,
    });
    logger = init.logger;
    if (cfg.defaulted) {
      logger.info("config file not found; using defaults");
    }
    if (init.prunedFiles.length > 0) {
      logger.debug({ prunedFiles: init.prunedFiles }, "pruned old log files");
    }

    step = "open_db";
    const dbPath = join(cfg.data_dir, "state.db");
    db = openDb(dbPath);

    step = "migrate";
    const result = await migrate(db, MIGRATIONS_DIR);
    if (result.applied.length > 0) {
      logger.info(
        { applied: result.applied, backupPath: result.backupPath },
        "applied migrations",
      );
    } else {
      logger.debug("no pending migrations");
    }

    logger.info(
      { version: VERSION, data_dir: cfg.data_dir },
      "ready",
    );

    // Keep the event loop alive until SIGINT/SIGTERM. `process.on('SIGINT')`
    // listeners are NOT active handles — without something keeping the loop
    // alive, Node exits as soon as boot resolves. This timer is the active
    // handle; the signal handlers call `process.exit(...)` which tears it
    // down along with everything else.
    setInterval(() => {}, 1 << 30);
  } catch (err) {
    const message = (err as Error).message ?? String(err);
    if (logger) {
      logger.error({ step, err: message }, "boot failed");
      try {
        (logger as unknown as { flush: () => void }).flush();
      } catch { /* ignore */ }
    } else {
      process.stderr.write(`boot failed at step "${step}": ${message}\n`);
    }
    try {
      db?.close();
    } catch { /* ignore */ }
    try {
      await release?.();
    } catch { /* ignore */ }
    process.exit(1);
  }
}

process.on("SIGINT", () => void shutdown("SIGINT"));
process.on("SIGTERM", () => void shutdown("SIGTERM"));

void boot();
