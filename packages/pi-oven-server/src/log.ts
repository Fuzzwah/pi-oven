import { mkdirSync, readdirSync, unlinkSync } from "node:fs";
import { join } from "node:path";
import pino, { type Logger, type StreamEntry } from "pino";
import pretty from "pino-pretty";
import type { LogLevel } from "./config.js";

export interface InitLoggerOptions {
  data_dir: string;
  log_level: LogLevel;
  tz: string;
  /** Override the development detection (used by tests). */
  development?: boolean;
  /** How many daily files to keep. Defaults to 7. */
  retainDays?: number;
  /** Override "now" for date stamping (used by tests). */
  now?: Date;
}

export interface InitLoggerResult {
  logger: Logger;
  logFilePath: string;
  prunedFiles: string[];
}

const DAILY_FILE_RE = /^server-(\d{4}-\d{2}-\d{2})\.ndjson$/;

function dateStampInTz(tz: string, now: Date): string {
  // en-CA produces ISO-style YYYY-MM-DD
  const fmt = new Intl.DateTimeFormat("en-CA", {
    timeZone: tz,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
  return fmt.format(now);
}

export function pruneOldLogs(dir: string, retain: number): string[] {
  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return [];
  }
  const dailies = entries
    .filter((name) => DAILY_FILE_RE.test(name))
    .sort()  // lex sort = chronological because of YYYY-MM-DD prefix
    .reverse(); // newest first

  const toDelete = dailies.slice(retain);
  const pruned: string[] = [];
  for (const name of toDelete) {
    try {
      unlinkSync(join(dir, name));
      pruned.push(name);
    } catch {
      /* best-effort prune */
    }
  }
  return pruned;
}

export function initLogger(opts: InitLoggerOptions): InitLoggerResult {
  const logsDir = join(opts.data_dir, "logs");
  mkdirSync(logsDir, { recursive: true });

  const retain = opts.retainDays ?? 7;
  const prunedFiles = pruneOldLogs(logsDir, retain);

  const stamp = dateStampInTz(opts.tz, opts.now ?? new Date());
  const logFilePath = join(logsDir, `server-${stamp}.ndjson`);

  const fileDest = pino.destination({
    dest: logFilePath,
    sync: false,
    mkdir: false,
    append: true,
  });

  const streams: StreamEntry[] = [
    { level: opts.log_level, stream: fileDest },
  ];

  const isDev =
    opts.development ?? process.env.NODE_ENV === "development";
  if (isDev) {
    const prettyStream = pretty({ colorize: true, sync: false });
    streams.push({ level: opts.log_level, stream: prettyStream });
  }

  const logger = pino(
    {
      level: opts.log_level,
      timestamp: pino.stdTimeFunctions.epochTime,
    },
    pino.multistream(streams),
  );

  return { logger, logFilePath, prunedFiles };
}

export function childLogger(
  parent: Logger,
  bindings: Record<string, unknown>,
): Logger {
  return parent.child(bindings);
}
