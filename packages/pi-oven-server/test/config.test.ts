import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, writeFileSync, chmodSync, rmSync, mkdirSync } from "node:fs";
import { tmpdir, homedir } from "node:os";
import { join } from "node:path";
import { loadConfig, ConfigError, ConfigMissingFieldError } from "../src/config.js";

let workdir: string;

beforeEach(() => {
  workdir = mkdtempSync(join(tmpdir(), "pi-oven-config-"));
});

afterEach(() => {
  rmSync(workdir, { recursive: true, force: true });
});

// Helper to write a valid TOML with the required shared_key.
function writeToml(path: string, content: string) {
  writeFileSync(path, content);
  chmodSync(path, 0o600);
}

describe("loadConfig", () => {
  it("returns defaults when the config file does not exist", () => {
    const cfg = loadConfig({
      configPath: join(workdir, "missing.toml"),
      env: { PI_OVEN_SHARED_KEY: "testkey" },
    });
    expect(cfg.data_dir).toBe(join(homedir(), ".pi-oven"));
    expect(cfg.log_level).toBe("info");
    expect(cfg.tz).toBe("UTC");
    expect(cfg.defaulted).toBe(true);
  });

  it("reads values from the TOML file", () => {
    const path = join(workdir, "server.toml");
    writeToml(
      path,
      'data_dir = "/tmp/pi-oven-test"\nlog_level = "debug"\ntz = "Australia/Brisbane"\n[net]\nshared_key = "filekey"\n',
    );

    const cfg = loadConfig({ configPath: path, env: {} });
    expect(cfg.data_dir).toBe("/tmp/pi-oven-test");
    expect(cfg.log_level).toBe("debug");
    expect(cfg.tz).toBe("Australia/Brisbane");
    expect(cfg.defaulted).toBe(false);
  });

  it("environment variables override file values", () => {
    const path = join(workdir, "server.toml");
    writeToml(path, 'log_level = "info"\ntz = "UTC"\n[net]\nshared_key = "filekey"\n');

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
    writeToml(path, 'data_dir = "~/custom-pi-oven"\n[net]\nshared_key = "filekey"\n');

    const cfg = loadConfig({ configPath: path, env: {} });
    expect(cfg.data_dir).toBe(join(homedir(), "custom-pi-oven"));
  });

  it("refuses to start when config file has loose permissions", () => {
    const path = join(workdir, "server.toml");
    writeFileSync(path, '[net]\nshared_key = "filekey"\n');
    chmodSync(path, 0o644); // group/other readable

    expect(() => loadConfig({ configPath: path, env: {} })).toThrow(ConfigError);
    expect(() => loadConfig({ configPath: path, env: {} })).toThrow(
      /insecure permissions/,
    );
  });

  it("throws on invalid log_level value in file", () => {
    const path = join(workdir, "server.toml");
    writeToml(path, 'log_level = "loud"\n[net]\nshared_key = "k"\n');

    expect(() => loadConfig({ configPath: path, env: {} })).toThrow(/invalid log_level/);
  });

  it("throws on invalid log_level via env", () => {
    expect(() =>
      loadConfig({
        configPath: join(workdir, "missing.toml"),
        env: { PI_OVEN_LOG_LEVEL: "loud", PI_OVEN_SHARED_KEY: "k" },
      }),
    ).toThrow(/invalid log_level/);
  });

  it("throws on invalid timezone in file", () => {
    const path = join(workdir, "server.toml");
    writeToml(path, 'tz = "Mars/Olympus"\n[net]\nshared_key = "k"\n');

    expect(() => loadConfig({ configPath: path, env: {} })).toThrow(/invalid tz/);
  });

  it("throws on invalid timezone via env", () => {
    expect(() =>
      loadConfig({
        configPath: join(workdir, "missing.toml"),
        env: { PI_OVEN_TZ: "Mars/Olympus", PI_OVEN_SHARED_KEY: "k" },
      }),
    ).toThrow(/invalid tz/);
  });
});

describe("[net] config (tasks 3.1–3.5)", () => {
  it("defaults: listen_addr=127.0.0.1:7878, origin_allowlist=[], allow_null_origin=true", () => {
    const cfg = loadConfig({
      configPath: join(workdir, "missing.toml"),
      env: { PI_OVEN_SHARED_KEY: "testkey" },
    });
    expect(cfg.net.listen_addr).toBe("127.0.0.1:7878");
    expect(cfg.net.origin_allowlist).toEqual([]);
    expect(cfg.net.allow_null_origin).toBe(true);
    expect(cfg.net.shared_key).toBe("testkey");
  });

  it("file-only key", () => {
    const path = join(workdir, "server.toml");
    writeToml(path, '[net]\nshared_key = "from-file"\n');

    const cfg = loadConfig({ configPath: path, env: {} });
    expect(cfg.net.shared_key).toBe("from-file");
  });

  it("env-only key", () => {
    const cfg = loadConfig({
      configPath: join(workdir, "missing.toml"),
      env: { PI_OVEN_SHARED_KEY: "from-env" },
    });
    expect(cfg.net.shared_key).toBe("from-env");
  });

  it("file-overrides-env: config file shared_key wins when both are set", () => {
    const path = join(workdir, "server.toml");
    writeToml(path, '[net]\nshared_key = "from-file"\n');

    const cfg = loadConfig({
      configPath: path,
      env: { PI_OVEN_SHARED_KEY: "from-env" },
    });
    expect(cfg.net.shared_key).toBe("from-file");
  });

  it("both-missing throws ConfigMissingFieldError with field=shared_key", () => {
    expect(() =>
      loadConfig({
        configPath: join(workdir, "missing.toml"),
        env: {},
      }),
    ).toThrow(ConfigMissingFieldError);

    try {
      loadConfig({ configPath: join(workdir, "missing.toml"), env: {} });
    } catch (err) {
      expect(err).toBeInstanceOf(ConfigMissingFieldError);
      expect((err as ConfigMissingFieldError).field).toBe("shared_key");
    }
  });

  it("reads listen_addr and origin_allowlist from [net] table", () => {
    const path = join(workdir, "server.toml");
    writeToml(
      path,
      '[net]\nshared_key = "k"\nlisten_addr = "0.0.0.0:9000"\norigin_allowlist = ["https://app.example.com"]\nallow_null_origin = false\n',
    );

    const cfg = loadConfig({ configPath: path, env: {} });
    expect(cfg.net.listen_addr).toBe("0.0.0.0:9000");
    expect(cfg.net.origin_allowlist).toEqual(["https://app.example.com"]);
    expect(cfg.net.allow_null_origin).toBe(false);
  });
});
