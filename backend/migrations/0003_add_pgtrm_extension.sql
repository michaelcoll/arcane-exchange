CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX idx_mv_card_prices_name_trgm
    ON mv_card_prices USING GIN (name gin_trgm_ops);