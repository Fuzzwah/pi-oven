## Requirements

### Requirement: Server entry point boot sequence

The server SHALL execute its boot sequence in this exact order before logging "ready": load configuration, acquire the single-instance lock, initialise the structured logger, open the SQLite database with required pragmas, and run the migration runner.

#### Scenario: Successful boot

- **WHEN** the server starts with valid config, no other instance running, and a healthy data directory
- **THEN** the server completes config load, lock acquire, logger init, DB open, and migration application in that order
- **AND** the server emits a final structured log line at level `info` with message `"ready"` and fields `{ pid, version, data_dir }`
- **AND** the server process remains alive (it does not exit after the ready line) until it receives `SIGINT` or `SIGTERM`

#### Scenario: Boot failure stops before ready

- **WHEN** any boot step fails (config invalid, lock held, DB unreadable, migration error)
- **THEN** the server logs an `error` line naming the failed step and exits with non-zero status
- **AND** the server does NOT emit the `"ready"` log line

### Requirement: Configuration loading from TOML with environment overrides

The server SHALL load configuration from `~/.pi-oven/server.toml`, then apply environment-variable overrides for any matching key. Missing config file SHALL result in a default config being used; environment overrides apply regardless.

#### Scenario: Config file present and parseable

- **WHEN** `~/.pi-oven/server.toml` exists with `log_level = "debug"` and `tz = "Australia/Brisbane"`
- **THEN** the server uses `debug` as its log level and `Australia/Brisbane` as its timezone

#### Scenario: Environment variable overrides file value

- **WHEN** `~/.pi-oven/server.toml` sets `log_level = "info"` and the environment variable `PI_OVEN_LOG_LEVEL` is set to `debug`
- **THEN** the server uses `debug` as its log level

#### Scenario: Config file missing uses defaults

- **WHEN** `~/.pi-oven/server.toml` does not exist
- **THEN** the server starts with default config (`data_dir = "~/.pi-oven"`, `log_level = "info"`, `tz = "UTC"`) and logs that defaults are in use

#### Scenario: Config file with insecure permissions refuses to start

- **WHEN** `~/.pi-oven/server.toml` exists with file mode looser than `0600` (e.g. world-readable)
- **THEN** the server logs an `error` describing the permission and exits non-zero before reading any value from the file

### Requirement: Single-instance lock on the data directory

The server SHALL hold an exclusive lock on `<data_dir>/server.lock` for the duration of its run. It SHALL NOT proceed past the lock step if another process holds the lock.

#### Scenario: Lock is free at startup

- **WHEN** no other server instance is running and `<data_dir>/server.lock` is unheld
- **THEN** the server acquires the lock, writes its PID and start timestamp into the lock file body, and continues boot

#### Scenario: Lock is held by another instance

- **WHEN** another server instance already holds `<data_dir>/server.lock`
- **THEN** the second server logs an `error` line including the holding PID (read from the lock file body) and exits non-zero
- **AND** the second server does NOT modify any file under `<data_dir>`

#### Scenario: Lock is released on shutdown

- **WHEN** a running server receives `SIGINT` or `SIGTERM`
- **THEN** the server releases the lock and exits cleanly
- **AND** a subsequent server instance can acquire the lock without manual intervention

### Requirement: Structured logging via pino with daily rotation

The server SHALL use pino as its logger, write NDJSON lines to `<data_dir>/logs/server-<YYYY-MM-DD>.ndjson`, retain the last 7 daily logs, and prune older files. Every log line SHALL be valid JSON with at minimum `level`, `time`, `pid`, and `msg` fields.

#### Scenario: Log line is valid NDJSON

- **WHEN** the server logs at any level
- **THEN** the resulting line in the daily log file parses as a JSON object containing `level`, `time` (unix ms), `pid`, and `msg`

#### Scenario: Log rotation by date

- **WHEN** the server is running across midnight in its configured timezone
- **THEN** subsequent log lines are written to the new `server-<next-date>.ndjson` file
- **AND** the previous day's file is left intact

#### Scenario: Old logs pruned

- **WHEN** the logger initialises and finds more than 7 daily log files in `<data_dir>/logs/`
- **THEN** the logger deletes files older than the 7th most recent before continuing

#### Scenario: Development pretty-printing

- **WHEN** the server starts with `NODE_ENV=development`
- **THEN** in addition to the file output, formatted log lines are written to stdout for developer convenience

### Requirement: SQLite database opened with required pragmas

The server SHALL open `<data_dir>/state.db` exactly once during boot, set required pragmas before any other query, and reuse this single connection for all subsequent operations. Every code path that opens this database file SHALL go through the shared open function so the pragmas are not bypassable.

#### Scenario: Pragmas applied on open

- **WHEN** the server opens `<data_dir>/state.db`
- **THEN** before any other SQL executes, the connection has applied: `journal_mode = WAL`, `synchronous = NORMAL`, `foreign_keys = ON`, `busy_timeout = 5000`, `temp_store = MEMORY`

#### Scenario: Database file is created if missing

- **WHEN** the server starts and `<data_dir>/state.db` does not exist
- **THEN** the file is created with mode `0600` and pragmas are applied to the new database
- **AND** the migration runner then creates the `_migrations` table

#### Scenario: Database file is unreadable

- **WHEN** the server starts and `<data_dir>/state.db` exists but cannot be opened (corrupt, wrong format, permission denied)
- **THEN** the server logs an `error` describing the failure and exits non-zero
- **AND** no automatic recovery is attempted

### Requirement: Graceful shutdown on SIGINT and SIGTERM

The server SHALL handle `SIGINT` and `SIGTERM` by flushing logs, closing the database connection, releasing the single-instance lock, and exiting with status zero.

#### Scenario: Clean shutdown flushes state

- **WHEN** the server receives `SIGINT` or `SIGTERM`
- **THEN** all buffered log lines are flushed to disk
- **AND** the SQLite connection is closed (committing any open transaction)
- **AND** the lock file is released
- **AND** the process exits with status zero

#### Scenario: Hard shutdown is detectable on next boot

- **WHEN** the server is killed with `SIGKILL` and a subsequent server starts
- **THEN** SQLite's WAL is recovered automatically by the new process
- **AND** the lock file from the killed process does not block the new server (kernel-released by `flock`)
