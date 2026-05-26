-- V005: drop the dead `sensitive` column from clipboard_items.
--
-- The column has been NOT NULL DEFAULT 0 since V002 and was never written
-- (no production query reads or writes it). Sensitive-clipboard exclusion
-- is now enforced at the watcher — content is dropped before it ever
-- reaches this table — so the column is unreachable. SQLite 3.35+
-- supports ALTER TABLE ... DROP COLUMN directly.
ALTER TABLE clipboard_items DROP COLUMN sensitive;
