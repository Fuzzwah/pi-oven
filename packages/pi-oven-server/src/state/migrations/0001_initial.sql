CREATE TABLE _migrations (
  id          INTEGER PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  checksum    TEXT NOT NULL,
  applied_at  INTEGER NOT NULL
);
