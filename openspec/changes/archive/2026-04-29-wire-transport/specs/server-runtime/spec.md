## MODIFIED Requirements

### Requirement: Server entry point boot sequence

The server SHALL execute its boot sequence in this exact order before logging "ready": load configuration, acquire the single-instance lock, initialise the structured logger, open the SQLite database with required pragmas, run the migration runner, and bind the WebSocket listener to the configured `listen_addr`.

#### Scenario: Successful boot

- **WHEN** the server starts with valid config, no other instance running, a healthy data directory, and a free `listen_addr`
- **THEN** the server completes config load, lock acquire, logger init, DB open, migration application, and listener bind in that order
- **AND** the server emits a final structured log line at level `info` with message `"ready"` and fields `{ pid, version, data_dir, listen_addr }`
- **AND** the WebSocket listener is accepting upgrade requests after the `"ready"` line is emitted
- **AND** the server process remains alive (it does not exit after the ready line) until it receives `SIGINT` or `SIGTERM`

#### Scenario: Boot failure stops before ready

- **WHEN** any boot step fails (config invalid, lock held, DB unreadable, migration error, listener bind failure)
- **THEN** the server logs an `error` line naming the failed step and exits with non-zero status
- **AND** the server does NOT emit the `"ready"` log line
- **AND** if migration failed, the WebSocket listener is NOT bound (a half-migrated DB is never visible on the wire)
