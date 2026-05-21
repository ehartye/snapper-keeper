CREATE TABLE clipboard_items (
    id                  TEXT PRIMARY KEY,          -- uuid v7
    kind                TEXT NOT NULL,             -- 'text' or 'image'
    text_content        TEXT,                      -- inline for text <= 8KB
    file_path           TEXT,                      -- relative path for images / large text
    content_hash        TEXT NOT NULL,             -- sha256 for dedup
    source_app          TEXT,
    source_window_title TEXT,
    created_at          INTEGER NOT NULL,          -- unix ms
    pinned              INTEGER NOT NULL DEFAULT 0,
    sensitive           INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_clipboard_items_created_at ON clipboard_items(created_at DESC);
CREATE INDEX idx_clipboard_items_hash ON clipboard_items(content_hash);
