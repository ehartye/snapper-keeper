-- Phase 8 — annotation editor state.
-- Stores the editable annotation state (shapes + crop) as JSON so users
-- can re-open a previously-edited capture and continue editing rather
-- than starting from scratch. NULL means the capture has never been
-- annotated under this system (legacy rows + brand-new captures).

ALTER TABLE captures ADD COLUMN annotation_state TEXT;
