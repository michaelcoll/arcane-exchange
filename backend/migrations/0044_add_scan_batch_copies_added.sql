-- What the first validation of a scan batch answered, so that sending the same batch again answers
-- the same number of copies. The copies of the batches recorded before this column are not known.
ALTER TABLE scan_batch
    ADD COLUMN copies_added INTEGER NOT NULL DEFAULT 0;
ALTER TABLE scan_batch
    ALTER COLUMN copies_added DROP DEFAULT;
