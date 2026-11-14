-- The date of the first card printed in the set, filled by the catalog update: the set selector
-- lists the most recent sets first. A set no catalog update has seen has none.
ALTER TABLE set_name
    ADD COLUMN released_at DATE;
