-- The URL of the Scryfall image of the catalog's reference printing (the front face of a
-- double-faced card), returned as is by the search: the client loads it from the Scryfall CDN.
-- Null for the rows already loaded, until the next catalog load.
ALTER TABLE catalog_card
    ADD COLUMN image_url TEXT;

-- The catalog search by name (English or French): ILIKE and trigram similarity.
CREATE INDEX idx_catalog_card_name_trgm ON catalog_card USING GIN (name gin_trgm_ops);
CREATE INDEX idx_catalog_card_french_name_trgm ON catalog_card USING GIN (french_name gin_trgm_ops);
