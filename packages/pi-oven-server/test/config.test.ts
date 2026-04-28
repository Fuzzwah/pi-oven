import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, writeFileSync, chmodSync, rmSync, mkdirSync } from "node:fs";
import { tmpdir, homedir } from "node:os";
import { join } from "node:path";
import { loadConfig, ConfigError } from "../src/config.js";

let workdir: string;

beforeEach(() => {
  workdir = mkdtempSync(join(tmpdir(), "pi-oven-config-"));
});

afterEach(() => {
  rmSync(workdir, { recursive: true, force: true });
});

describe("loadConfig", () => {
  it("returns defaults when the config file does not exist", () => {
    const cfg = loadConfig({
      configPath: join(workdir, "missing.toml"),
      env: {},
    });
    expect(cfg.data_dir).toBe(join(homedir(), ".pi-oven"));
    expect(cfg.log_level).toBe("info");
    expect(cfg.tz).toBe("UTC");
    expect(cfg.defaulted).toBe(true);
  });

  it("reads values from the TOML file", () => {
    const path = join(workdir, "server.toml");
    writeFileSync(
      path,
      'data_dir = "/tmp/pi-oven-test"\nlog_level = "debug"\ntz = "Australia/Brisbane"\n',
    );
    chmodSync(path, 0o600);

    const cfg = loadConfig({ configPath: path, env: {} });
    expect(cfg.data_dir).toBe("/tmp/pi-oven-test");
    expect(cfg.log_level).toBe("debug");
    expect(cfg.tz).toBe("Australia/Brisbane");
    expect(cfg.defaulted).toBe(false);
  });

  it("environment variables override file values", () => {
    const path = join(workdir, "server.toml");
    writeFileSync(path, 'log_level = "info"\ntz = "UTC"\n');
    chmodSync(path, 0o600);

    const cfg = loadConfig({
      configPath: path,
      env: {
        PI_OVEN_LOG_LEVEL: "debug",
        PI_OVEN_TZ: "Australia/Sydney",
      },
    });
    expect(cfg.log_level).toBe("debug");
    expect(cfg.tz).toBe("Australia/Sydney");
  });

  it("expands ~ in data_dir using os.homedir()", () => {
    const path = join(workdir, "server.toml");
    writeFileSync(path, 'data_dir = "~/custom-pi-oven"\n');
    chmodSync(path, 0o600);

    const cfg = loadConfig({ configPath: path, env: {} });
    expect(cfg.data_dir).toBe(join(homedir(), "custom-pi-oven"));
  });

  it("refuses to start when config file has loose permissions", () => {
    const path = join(workdir, "server.toml");
    writeFileSync(path, 'log_level = "info"\n');
    chmodSync(path, 0o644); // group/other readable

    expect(() => loadConfig({ configPath: path, env: {} })).toThrow(ConfigError);
    expect(() => loadConfig({ configPath: path, env: {} })).toThrow(
      /insecure permissions/,
    );
  });

  it("throws on invalid log_level value in file", () => {
    const path = join(workdir, "server.toml");
    writeFileSync(path, 'log_level = "loud"\n');
    chmodSync(path, 0o600);

    expect(() => loadConfig({ configPath: path, env: {} })).toThrow(/invalid log_level/);
  });

  it("throws on invalid log_level via env", () => {
    expect(() =>
      loadConfig({
        configPath: join(workdir, "missing.toml"),
        env: { PI_OVEN_LOG_LEVEL: "loud" },
      }),
    ).toThrow(/invalid log_level/);
  });
});
