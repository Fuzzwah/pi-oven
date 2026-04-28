import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));

export const PACKAGE_ROOT = resolve(__dirname, "..");
export const MIGRATIONS_DIR = join(PACKAGE_ROOT, "src", "state", "migrations");
