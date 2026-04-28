import { acquireLock, LockError } from "../../src/lock.js";

const dir = process.argv[2];
if (!dir) {
  console.error("usage: hold-lock.ts <dir>");
  process.exit(2);
}

try {
  const release = await acquireLock(dir, { registerProcessHandlers: false });
  // Print holder pid and exit successfully — the test arranges for it never to reach this branch
  // when the lock is already held.
  process.stdout.write(`ACQUIRED:${process.pid}\n`);
  await release();
  process.exit(0);
} catch (err) {
  if (err instanceof LockError) {
    process.stderr.write(`LOCKED_BY:${err.holderPid ?? "unknown"}\n`);
    process.stderr.write(err.message + "\n");
    process.exit(1);
  }
  process.stderr.write(`UNEXPECTED:${(err as Error).message}\n`);
  process.exit(2);
}
