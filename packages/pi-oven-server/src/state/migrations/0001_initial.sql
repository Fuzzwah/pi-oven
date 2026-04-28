-- The migration runner pre-creates this table idempotently so the SELECT can
-- succeed on a fresh DB. Using IF NOT EXISTS here means re-running 0001 from
-- scratch (or against a DB the runner already touched) is a no-op rather than
-- a hard failure, which matches the runner's pre-create behaviour.
CREATE TABLE IF NOT EXISTS _migrations (
  id          INTEGER PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  checksum    TEXT NOT NULL,
  applied_at  INTEGER NOT NULL
);
