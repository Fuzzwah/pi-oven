## Requirements

### Requirement: Forward-only numbered migration files

Migration files SHALL live in `packages/pi-oven-server/src/state/migrations/`, be named with a zero-padded four-digit numeric prefix followed by an underscore and a kebab-case description, and execute in lexicographic filename order. The migration system SHALL NOT support `down` migrations.

#### Scenario: Numeric prefix determines order

- **WHEN** the migrations directory contains `0001_initial.sql`, `0002_add_projects.sql`, `0010_rename_branch.sql`
- **THEN** the runner applies them in the order `0001` → `0002` → `0010`

#### Scenario: SQL and TypeScript migrations are both supported

- **WHEN** the migrations directory contains a mix of `.sql` and `.ts` files (e.g. `0001_initial.sql`, `0002_seed_data.ts`)
- **THEN** both file types are applied in filename order
- **AND** `.sql` files are executed via `db.exec()`, `.ts` files are dynamically imported and their `up(db)` export is called

#### Scenario: No down migrations

- **WHEN** a developer attempts to roll back a migration
- **THEN** there is no `down`/`rollback` command provided by the runner
- **AND** the documented rollback procedure is "stop server, restore the timestamped backup, downgrade binary"

### Requirement: Migration tracking table with checksums

The migration runner SHALL track applied migrations in a `_migrations` table within the same database, recording the filename, a SHA-256 checksum of the file's bytes at apply time, and the application timestamp.

#### Scenario: Table created on first run

- **WHEN** the runner executes against a database without a `_migrations` table
- **THEN** the runner creates the `_migrations` table with columns `id INTEGER PRIMARY KEY`, `name TEXT NOT NULL UNIQUE`, `checksum TEXT NOT NULL`, `applied_at INTEGER NOT NULL`

#### Scenario: Each applied migration recorded

- **WHEN** the runner successfully applies migration `0002_add_projects.sql`
- **THEN** a row is inserted into `_migrations` with `name = "0002_add_projects.sql"`, `checksum = sha256(file bytes)`, and `applied_at = current unix milliseconds`

### Requirement: Refusal to start on tampered checksums

If the runner finds an entry in `_migrations` whose corresponding file on disk has a different SHA-256 checksum than the recorded one, the runner SHALL refuse to start and SHALL exit non-zero without applying any pending migrations or modifying any data.

#### Scenario: Checksum matches

- **WHEN** every applied migration's file on disk has a checksum matching the `_migrations` row
- **THEN** the runner proceeds to apply pending migrations (or no-ops if none)

#### Scenario: Checksum mismatch on applied migration

- **WHEN** an applied migration's file on disk has been edited so its SHA-256 differs from the `_migrations` row
- **THEN** the runner logs an `error` line naming the migration and the mismatch, exits non-zero, and does NOT apply any pending migrations

#### Scenario: Applied migration missing from disk

- **WHEN** an entry exists in `_migrations` but the corresponding file is not present in the migrations directory
- **THEN** the runner logs an `error` line naming the missing migration, exits non-zero, and does NOT apply any pending migrations

### Requirement: Atomic application of each migration

Each migration SHALL be applied within a single SQLite transaction (`BEGIN IMMEDIATE` … `COMMIT`). If any statement in the migration fails, the transaction SHALL be rolled back and the `_migrations` table SHALL NOT record the migration.

#### Scenario: Successful migration commits cleanly

- **WHEN** the runner applies a migration whose statements all succeed
- **THEN** the schema changes are visible in the database
- **AND** a corresponding row exists in `_migrations`

#### Scenario: Failing migration leaves no partial state

- **WHEN** the runner applies a migration whose final statement raises an error
- **THEN** none of the migration's statements remain in effect (rolled back)
- **AND** no row is inserted into `_migrations` for that migration
- **AND** the runner exits non-zero with an error line naming the failed migration
- **AND** subsequent boots will retry the same migration

### Requirement: Automatic backup before pending migrations

Before applying any pending migrations, the runner SHALL produce an atomic snapshot of the current database to `<state.db path>.bak.<unix-ms>` using SQLite's online backup API. The runner SHALL NOT take a backup if no migrations are pending. The runner SHALL retain the most recent 10 backups and prune older ones.

#### Scenario: Backup taken when migrations are pending

- **WHEN** the runner finds at least one pending migration
- **THEN** before applying any of them, it creates `<state.db path>.bak.<unix-ms>` containing a consistent snapshot of the current database
- **AND** the backup file's size is non-zero

#### Scenario: No backup taken when up to date

- **WHEN** the runner finds no pending migrations
- **THEN** no new backup file is created
- **AND** the runner exits the migration phase quickly (no SQL executed beyond reading `_migrations`)

#### Scenario: Old backups pruned

- **WHEN** more than 10 `state.db.bak.*` files exist after a successful migration run
- **THEN** the oldest backups beyond the 10 most recent are deleted

### Requirement: Server boot blocks until migrations succeed

The server SHALL NOT begin accepting any client connection (in this change: SHALL NOT log `"ready"`; in future changes: SHALL NOT open any listening socket) until the migration runner returns successfully.

#### Scenario: Migrations succeed before ready

- **WHEN** the server boots and the runner applies one or more pending migrations successfully
- **THEN** the server logs the `"ready"` line only after the runner has returned

#### Scenario: Migrations fail blocks ready

- **WHEN** the runner fails for any reason (checksum mismatch, missing file, transaction error)
- **THEN** the server logs an `error` line naming the failure and exits non-zero
- **AND** the `"ready"` log line is never written

### Requirement: Initial migration creates only the tracking table

The initial migration `0001_initial.sql` SHALL create the `_migrations` table and nothing else. Feature tables SHALL be added by subsequent numbered migrations introduced by the changes that need them.

#### Scenario: Initial migration scope

- **WHEN** a fresh database is migrated
- **THEN** after `0001_initial.sql` runs, the schema contains only the `_migrations` table (plus any SQLite internal tables)

### Requirement: Developer scripts for migration management

The server package SHALL expose npm scripts: `pnpm migrate:status` to print applied and pending migrations, `pnpm migrate:new <slug>` to scaffold the next-numbered migration file, and `pnpm migrate:reset` (development-only, requires typed confirmation) to delete the database and re-run all migrations.

#### Scenario: Status lists applied and pending

- **WHEN** a developer runs `pnpm migrate:status`
- **THEN** the output lists each applied migration (name, applied-at) and each pending migration (name)
- **AND** the exit status is zero regardless of whether migrations are pending

#### Scenario: New scaffolds next-numbered file

- **WHEN** a developer runs `pnpm migrate:new add-projects` and the highest existing migration is `0001_initial.sql`
- **THEN** an empty `0002_add-projects.sql` file is created in the migrations directory
- **AND** the file path is printed to stdout

#### Scenario: Reset requires confirmation in development only

- **WHEN** a developer runs `pnpm migrate:reset` in a development environment
- **THEN** the script prompts for typed confirmation (e.g. type the data directory path) before deleting the database
- **AND** if `NODE_ENV=production` (or the data directory contains any non-empty backup), the script refuses to run and exits non-zero
