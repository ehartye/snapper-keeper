-- Phase 5 — OCR text storage + FTS5 indexes for captures and clipboard.

CREATE TABLE ocr_text (
    capture_id  TEXT PRIMARY KEY REFERENCES captures(id) ON DELETE CASCADE,
    text        TEXT NOT NULL,
    language    TEXT NOT NULL DEFAULT 'eng',
    confidence  REAL NOT NULL DEFAULT 0.0,
    created_at  INTEGER NOT NULL  -- unix ms
);

-- Contentless-delete FTS5 — we own population explicitly.
-- contentless_delete=1 enables DELETE by rowid/UNINDEXED column (requires SQLite 3.43+;
-- rusqlite 0.31 bundles 3.45+). Without this flag, DELETE is not supported on contentless tables.
CREATE VIRTUAL TABLE captures_fts USING fts5(
    capture_id UNINDEXED,
    source_app,
    window_title,
    ocr_text,
    tag_names,
    content='',
    contentless_delete=1
);

CREATE VIRTUAL TABLE clipboard_fts USING fts5(
    clipboard_id UNINDEXED,
    text_content,
    source_app,
    window_title,
    content='',
    contentless_delete=1
);
