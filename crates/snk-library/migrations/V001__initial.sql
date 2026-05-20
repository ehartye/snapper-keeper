-- Phase 1 schema — captures, tags, settings.
-- FTS, clipboard, ocr_text added in later migrations.

CREATE TABLE captures (
    id              TEXT PRIMARY KEY,                          -- uuid v7
    file_path       TEXT NOT NULL,                             -- relative · captures/YYYY/MM/uuid.png
    annotated_path  TEXT,
    width           INTEGER NOT NULL,
    height          INTEGER NOT NULL,
    source_app      TEXT,
    source_window_title TEXT,
    monitor         TEXT,
    created_at      INTEGER NOT NULL,                          -- unix ms
    deleted_at      INTEGER,
    pinned          INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_captures_created_at ON captures(created_at DESC);
CREATE INDEX idx_captures_deleted_at ON captures(deleted_at) WHERE deleted_at IS NOT NULL;

CREATE TABLE tags (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    color      TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE capture_tags (
    capture_id TEXT NOT NULL REFERENCES captures(id) ON DELETE CASCADE,
    tag_id     TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (capture_id, tag_id)
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL  -- json
);

CREATE TABLE hotkey_bindings (
    action_id TEXT PRIMARY KEY,
    chord     TEXT NOT NULL
);
