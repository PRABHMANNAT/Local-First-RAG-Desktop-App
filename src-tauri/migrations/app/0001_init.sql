-- App-level registry database. One per installation. Holds the list of
-- workspaces and global app settings. Per-workspace content lives in each
-- workspace's own SQLite file (see migrations/workspace).

CREATE TABLE workspace (
    id          TEXT PRIMARY KEY,        -- uuidv7
    name        TEXT NOT NULL,
    dir         TEXT NOT NULL UNIQUE,    -- absolute path chosen by the user
    icon        TEXT,                    -- emoji or short token
    created_at  INTEGER NOT NULL         -- unix millis
);

CREATE TABLE app_setting (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL                  -- JSON-encoded value
);
