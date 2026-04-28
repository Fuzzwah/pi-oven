import {
  closeSync,
  existsSync,
  mkdirSync,
  openSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import lockfile from "proper-lockfile";

export interface LockMetadata {
  pid: number;
  started_at: number;
}

export class LockError extends Error {
  readonly holderPid?: number;
  readonly holderStartedAt?: number;
  readonly lockPath: string;
  constructor(
    message: string,
    lockPath: string,
    meta?: LockMetadata | undefined,
  ) {
    super(message);
    this.name = "LockError";
    this.lockPath = lockPath;
    this.holderPid = meta?.pid;
    this.holderStartedAt = meta?.started_at;
  }
}

export type ReleaseLock = () => Promise<void>;

export interface AcquireLockOptions {
  /** When true (the default), register exit / signal handlers to auto-release. */
  registerProcessHandlers?: boolean;
}

const LOCK_BASENAME = "server.lock";

function readMetadata(path: string): LockMetadata | undefined {
  try {
    const text = readFileSync(path, "utf8").trim();
    if (!text) return undefined;
    const parsed = JSON.parse(text) as Partial<LockMetadata>;
    if (
      typeof parsed.pid === "number" &&
      typeof parsed.started_at === "number"
    ) {
      return { pid: parsed.pid, started_at: parsed.started_at };
    }
  } catch {
    /* malformed body — treat as no metadata */
  }
  return undefined;
}

export async function acquireLock(
  dir: string,
  opts: AcquireLockOptions = {},
): Promise<ReleaseLock> {
  mkdirSync(dir, { recursive: true });
  const lockPath = join(dir, LOCK_BASENAME);

  if (!existsSync(lockPath)) {
    closeSync(openSync(lockPath, "w"));
  }

  let release: () => Promise<void>;
  try {
    release = await lockfile.lock(lockPath, { realpath: false, retries: 0 });
  } catch (err) {
    const e = err as NodeJS.ErrnoException;
    if (e.code === "ELOCKED") {
      const meta = readMetadata(lockPath);
      const pidPart = meta ? ` (held by pid ${meta.pid})` : "";
      throw new LockError(
        `${lockPath}: another pi-oven server is already running${pidPart}`,
        lockPath,
        meta,
      );
    }
    throw err;
  }

  const meta: LockMetadata = { pid: process.pid, started_at: Date.now() };
  writeFileSync(lockPath, JSON.stringify(meta));

  let released = false;
  const releaseFn: ReleaseLock = async () => {
    if (released) return;
    released = true;
    try {
      await release();
    } catch {
      /* lock dir already gone — ignore */
    }
  };

  if (opts.registerProcessHandlers !== false) {
    const onExit = () => {
      if (released) return;
      released = true;
      try {
        lockfile.unlockSync(lockPath, { realpath: false });
      } catch {
        /* best-effort sync release on exit */
      }
    };
    const onSignal = (signal: NodeJS.Signals) => {
      const code = signal === "SIGINT" ? 130 : 143;
      void releaseFn().finally(() => process.exit(code));
    };
    process.on("exit", onExit);
    process.on("SIGINT", () => onSignal("SIGINT"));
    process.on("SIGTERM", () => onSignal("SIGTERM"));
  }

  return releaseFn;
}
