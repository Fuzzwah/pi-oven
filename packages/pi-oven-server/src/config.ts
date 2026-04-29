import { homedir } from "node:os";
import { statSync, readFileSync } from "node:fs";
import { join } from "node:path";
import TOML from "@iarna/toml";

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error" | "fatal";

export interface ServerConfig {
  data_dir: string;
  log_level: LogLevel;
  tz: string;
  /** True when the config file was absent and defaults were used. */
  defaulted: boolean;
}

export interface LoadConfigOptions {
  /** Override the default `~/.pi-oven/server.toml` lookup (used by tests). */
  configPath?: string;
  /** Override `process.env` (used by tests). */
  env?: NodeJS.ProcessEnv;
}

const DEFAULT_DATA_DIR = "~/.pi-oven";
const DEFAULT_LOG_LEVEL: LogLevel = "info";
const DEFAULT_TZ = "UTC";

const VALID_LOG_LEVELS: ReadonlySet<LogLevel> = new Set([
  "trace",
  "debug",
  "info",
  "warn",
  "error",
  "fatal",
]);

export class ConfigError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ConfigError";
  }
}

function expandHome(p: string): string {
  if (p === "~") return homedir();
  if (p.startsWith("~/")) return join(homedir(), p.slice(2));
  return p;
}

function defaultConfigPath(env: NodeJS.ProcessEnv): string {
  return join(homedir(), ".pi-oven", "server.toml");
}

function asLogLevel(value: unknown, source: string): LogLevel {
  if (typeof value !== "string" || !VALID_LOG_LEVELS.has(value as LogLevel)) {
    throw new ConfigError(
      `${source}: invalid log_level "${String(value)}" (expected one of ${[...VALID_LOG_LEVELS].join(", ")})`,
    );
  }
  return value as LogLevel;
}

function asTimeZone(value: unknown, source: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new ConfigError(`${source}: invalid tz "${String(value)}"`);
  }

  try {
    new Intl.DateTimeFormat("en-CA", { timeZone: value });
  } catch (err) {
    throw new ConfigError(
      `${source}: invalid tz "${value}": ${(err as Error).message}`,
    );
  }

  return value;
}

export function loadConfig(opts: LoadConfigOptions = {}): ServerConfig {
  const env = opts.env ?? process.env;
  const configPath = opts.configPath ?? defaultConfigPath(env);

  let fileValues: Record<string, unknown> = {};
  let defaulted = true;

  try {
    const stat = statSync(configPath);
    // Permission check: refuse if any group or world bits are set
    if ((stat.mode & 0o077) !== 0) {
      const mode = (stat.mode & 0o777).toString(8).padStart(3, "0");
      throw new ConfigError(
        `${configPath}: insecure permissions (mode ${mode}); must be 0600 or stricter`,
      );
    }
    const text = readFileSync(configPath, "utf8");
    try {
      fileValues = TOML.parse(text) as Record<string, unknown>;
    } catch (err) {
      throw new ConfigError(
        `${configPath}: failed to parse TOML: ${(err as Error).message}`,
      );
    }
    defaulted = false;
  } catch (err: unknown) {
    if (err instanceof ConfigError) throw err;
    const e = err as NodeJS.ErrnoException;
    if (e.code !== "ENOENT") {
      throw new ConfigError(
        `${configPath}: cannot stat config file: ${e.message}`,
      );
    }
    // ENOENT — fall through with defaults
  }

  let data_dir =
    typeof fileValues.data_dir === "string"
      ? fileValues.data_dir
      : DEFAULT_DATA_DIR;
  let log_level: LogLevel =
    fileValues.log_level !== undefined
      ? asLogLevel(fileValues.log_level, configPath)
      : DEFAULT_LOG_LEVEL;
  let tz =
    fileValues.tz !== undefined ? asTimeZone(fileValues.tz, configPath) : DEFAULT_TZ;

  if (env.PI_OVEN_DATA_DIR !== undefined && env.PI_OVEN_DATA_DIR !== "") {
    data_dir = env.PI_OVEN_DATA_DIR;
  }
  if (env.PI_OVEN_LOG_LEVEL !== undefined && env.PI_OVEN_LOG_LEVEL !== "") {
    log_level = asLogLevel(env.PI_OVEN_LOG_LEVEL, "PI_OVEN_LOG_LEVEL");
  }
  if (env.PI_OVEN_TZ !== undefined && env.PI_OVEN_TZ !== "") {
    tz = asTimeZone(env.PI_OVEN_TZ, "PI_OVEN_TZ");
  }

  return {
    data_dir: expandHome(data_dir),
    log_level,
    tz,
    defaulted,
  };
}
