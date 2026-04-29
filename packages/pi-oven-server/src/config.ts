import { homedir } from "node:os";
import { statSync, readFileSync } from "node:fs";
import { join } from "node:path";
import TOML from "@iarna/toml";

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error" | "fatal";

export interface NetConfig {
  listen_addr: string;
  shared_key: string;
  origin_allowlist: string[];
  allow_null_origin: boolean;
}

export interface ServerConfig {
  data_dir: string;
  log_level: LogLevel;
  tz: string;
  net: NetConfig;
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
const DEFAULT_LISTEN_ADDR = "127.0.0.1:7878";

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

export class ConfigMissingFieldError extends ConfigError {
  constructor(public readonly field: string) {
    super(`required config field missing: ${field}`);
    this.name = "ConfigMissingFieldError";
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

  // [net] section
  const netTable =
    typeof fileValues.net === "object" && fileValues.net !== null
      ? (fileValues.net as Record<string, unknown>)
      : {};

  const listen_addr =
    typeof netTable.listen_addr === "string"
      ? netTable.listen_addr
      : DEFAULT_LISTEN_ADDR;

  const origin_allowlist = Array.isArray(netTable.origin_allowlist)
    ? netTable.origin_allowlist.filter((x): x is string => typeof x === "string")
    : [];

  const allow_null_origin =
    typeof netTable.allow_null_origin === "boolean"
      ? netTable.allow_null_origin
      : true;

  // shared_key: config file takes priority over env var (task 3.3)
  let shared_key: string | undefined;
  if (typeof netTable.shared_key === "string" && netTable.shared_key.length > 0) {
    shared_key = netTable.shared_key;
  } else if (
    typeof env.PI_OVEN_SHARED_KEY === "string" &&
    env.PI_OVEN_SHARED_KEY.length > 0
  ) {
    shared_key = env.PI_OVEN_SHARED_KEY;
  }

  if (!shared_key) {
    throw new ConfigMissingFieldError("shared_key");
  }

  return {
    data_dir: expandHome(data_dir),
    log_level,
    tz,
    defaulted,
    net: { listen_addr, shared_key, origin_allowlist, allow_null_origin },
  };
}
