-- Phase 10 — Per-word bounds + engine version on ocr_text; PII spans table.

ALTER TABLE ocr_text ADD COLUMN words_json TEXT;
ALTER TABLE ocr_text ADD COLUMN engine     TEXT NOT NULL DEFAULT '';

CREATE TABLE pii_spans (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    capture_id   TEXT    NOT NULL REFERENCES captures(id) ON DELETE CASCADE,
    category     TEXT    NOT NULL,
    matched_text TEXT    NOT NULL,
    bbox_x       REAL    NOT NULL,
    bbox_y       REAL    NOT NULL,
    bbox_w       REAL    NOT NULL,
    bbox_h       REAL    NOT NULL,
    confidence   REAL    NOT NULL,
    redacted_at  INTEGER,
    dismissed_at INTEGER,
    created_at   INTEGER NOT NULL
);

CREATE INDEX idx_pii_spans_capture ON pii_spans(capture_id);
CREATE INDEX idx_pii_spans_pending ON pii_spans(capture_id)
    WHERE redacted_at IS NULL AND dismissed_at IS NULL;
