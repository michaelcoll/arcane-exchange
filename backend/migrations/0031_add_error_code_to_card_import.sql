-- Stable code of a failed import's error, translated by the clients (ADR 0018). Imports failed
-- before this column existed keep a NULL code, which clients show as a generic failure.
ALTER TABLE card_import
    ADD COLUMN error_code TEXT NULL;
