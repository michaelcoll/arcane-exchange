-- A collection entry is designated by an identifier in the API (ADR-0022), to edit or remove it.
-- Existing rows get theirs from the default; an import writes fresh ones, which is acceptable
-- since it replaces the whole collection.
ALTER TABLE collection_entry
    ADD COLUMN id UUID NOT NULL DEFAULT gen_random_uuid();
ALTER TABLE collection_entry
    ADD PRIMARY KEY (id);
