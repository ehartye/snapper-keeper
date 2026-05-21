-- Phase 5 — OCR text storage + FTS5 indexes for captures and clipboard.

CREATE TABLE ocr_text (
    capture_id  TEXT PRIMARY KEY REFERENCES captures(id) ON DELETE CASCADE,
    text        TEXT NOT NULL,
    language    TEXT NOT NULL DEFAULT 'eng',
    confidence  REAL NOT NULL DEFAULT 0.0,
    created_at  INTEGER NOT NULL  -- unix ms
);

-- Regular FTS5 (not contentless). Contentless mode (content='') was considered but
-- returns NULL for every column except rowid on SELECT, including UNINDEXED columns —
-- making our capture_id/clipboard_id lookup pattern unworkable. The contentless_unindexed=1
-- option fixes that but requires SQLite 3.47+; libsqlite3-sys 0.28 / rusqlite 0.31 bundles
-- SQLite 3.45. Regular FTS5 stores the indexed text columns (small strings) and supports
-- DELETE natively.
CREATE VIRTUAL TABLE captures_fts USING fts5(
    capture_id UNINDEXED,
    source_app,
    window_title,
    ocr_text,
    tag_names
);

CREATE VIRTUAL TABLE clipboard_fts USING fts5(
    clipboard_id UNINDEXED,
    text_content,
    source_app,
    window_title
);
