import { readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { MIGRATIONS_DIR } from "./_paths.js";

const FILE_RE = /^(\d{4})_.*\.(sql|ts)$/;

function normaliseSlug(raw: string): string {
  const cleaned = raw
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  if (cleaned.length === 0) {
    throw new Error(`slug "${raw}" is empty after normalisation`);
  }
  return cleaned;
}

function nextNumber(): string {
  let highest = 0;
  let entries: string[];
  try {
    entries = readdirSync(MIGRATIONS_DIR);
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === "ENOENT") entries = [];
    else throw err;
  }
  for (const name of entries) {
    const m = name.match(FILE_RE);
    if (m) {
      const n = Number(m[1]);
      if (n > highest) highest = n;
    }
  }
  const next = highest + 1;
  return next.toString().padStart(4, "0");
}

async function main(): Promise<void> {
  const slugRaw = process.argv[2];
  if (!slugRaw) {
    console.error("usage: pnpm migrate:new <slug>");
    process.exit(2);
  }
  const slug = normaliseSlug(slugRaw);
  const num = nextNumber();
  const filename = `${num}_${slug}.sql`;
  const path = join(MIGRATIONS_DIR, filename);

  const body = `-- migration: ${filename}\n-- describe what this migration does\n`;
  writeFileSync(path, body, { flag: "wx" });
  console.log(path);
}

await main();
