import {
  createWriteStream,
  mkdirSync,
  openSync,
  readSync,
  fstatSync,
  closeSync,
  statSync,
  readdirSync,
  createReadStream,
} from "node:fs";
import { createInterface } from "node:readline";
import type { WriteStream } from "node:fs";
import { join } from "node:path";

const MAX_FILE_SIZE = 64 * 1024 * 1024; // 64 MB

export interface LogEntry {
  seq: number;
  ts: number;
  event: unknown;
}

export class EventLog {
  private readonly dir: string;
  private currentStream: WriteStream | null = null;
  private currentFile: string | null = null;
  private currentSize: number = 0;
  private rotationIdx: number = 0;
  nextSeq: number;

  private constructor(dir: string, nextSeq: number, rotationIdx: number) {
    this.dir = dir;
    this.nextSeq = nextSeq;
    this.rotationIdx = rotationIdx;
  }

  static async open(dir: string): Promise<EventLog> {
    const files = EventLog.listLogFiles(dir);
    let nextSeq = 1;
    let rotationIdx = 0;
    if (files.length > 0) {
      const lastFile = files[files.length - 1]!;
      const lastSeq = EventLog.readLastSeq(join(dir, lastFile));
      if (lastSeq !== null) {
        nextSeq = lastSeq + 1;
      }
      const match = lastFile.match(/-(\d+)\.ndjson$/);
      if (match) {
        rotationIdx = parseInt(match[1]!, 10);
      }
    }
    return new EventLog(dir, nextSeq, rotationIdx);
  }

  static listLogFiles(dir: string): string[] {
    try {
      const entries = readdirSync(dir);
      return entries.filter((f) => f.endsWith(".ndjson")).sort();
    } catch {
      return [];
    }
  }

  private static readLastSeq(filePath: string): number | null {
    try {
      const fd = openSync(filePath, "r");
      const stat = fstatSync(fd);
      const size = stat.size;
      if (size === 0) {
        closeSync(fd);
        return null;
      }
      const readSize = Math.min(4096, size);
      const buf = Buffer.allocUnsafe(readSize);
      readSync(fd, buf, 0, readSize, size - readSize);
      closeSync(fd);
      const text = buf.toString("utf8");
      const lines = text.split("\n").filter((l) => l.trim().length > 0);
      const lastLine = lines[lines.length - 1];
      if (!lastLine) return null;
      const parsed = JSON.parse(lastLine) as Record<string, unknown>;
      return typeof parsed.seq === "number" ? parsed.seq : null;
    } catch {
      return null;
    }
  }

  async append(event: unknown): Promise<number> {
    const seq = this.nextSeq++;
    const entry: LogEntry = { seq, ts: Date.now(), event };
    const line = JSON.stringify(entry) + "\n";

    await this.ensureStream();

    await new Promise<void>((resolve, reject) => {
      this.currentStream!.write(line, (err) => {
        if (err) reject(err);
        else resolve();
      });
    });

    this.currentSize += Buffer.byteLength(line, "utf8");

    if (this.currentSize >= MAX_FILE_SIZE) {
      await this.rotate();
    }

    return seq;
  }

  private async ensureStream(): Promise<void> {
    if (this.currentStream) return;

    mkdirSync(this.dir, { recursive: true, mode: 0o700 });

    const createdAt = Date.now();
    const filename = `${createdAt}-${this.rotationIdx}.ndjson`;
    this.currentFile = join(this.dir, filename);

    await new Promise<void>((resolve, reject) => {
      const stream = createWriteStream(this.currentFile!, {
        flags: "a",
        mode: 0o600,
      });
      stream.once("open", () => resolve());
      stream.once("error", reject);
      this.currentStream = stream;
    });

    try {
      const stat = statSync(this.currentFile);
      this.currentSize = stat.size;
    } catch {
      this.currentSize = 0;
    }
  }

  private async rotate(): Promise<void> {
    if (this.currentStream) {
      await new Promise<void>((resolve) => {
        this.currentStream!.end(resolve);
      });
      this.currentStream = null;
    }
    this.rotationIdx++;
    const createdAt = Date.now();
    const filename = `${createdAt}-${this.rotationIdx}.ndjson`;
    this.currentFile = join(this.dir, filename);

    await new Promise<void>((resolve, reject) => {
      const stream = createWriteStream(this.currentFile!, {
        flags: "a",
        mode: 0o600,
      });
      stream.once("open", () => resolve());
      stream.once("error", reject);
      this.currentStream = stream;
    });
    this.currentSize = 0;
  }

  async *replay(lastSeq: number): AsyncGenerator<LogEntry> {
    const files = EventLog.listLogFiles(this.dir);

    for (const filename of files) {
      const filePath = join(this.dir, filename);
      const rl = createInterface({
        input: createReadStream(filePath),
        crlfDelay: Infinity,
      });

      for await (const line of rl) {
        if (!line.trim()) continue;
        try {
          const entry = JSON.parse(line) as LogEntry;
          if (entry.seq > lastSeq) {
            yield entry;
          }
        } catch {
          // skip malformed lines
        }
      }
    }
  }

  async close(): Promise<void> {
    if (this.currentStream) {
      await new Promise<void>((resolve) => {
        this.currentStream!.end(resolve);
      });
      this.currentStream = null;
    }
  }
}
