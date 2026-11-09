-- ADR-0023 / #671: a copy can have no purchase price. The import turned an empty price into 0, so
-- the two can no longer be told apart, and a price of 0 already gave no gap since the purchase.
-- The existing prices of 0 therefore become absent.
--
-- `mv_card_prices` reads the column: it is recreated (definition of 0034), with an average
-- weighted over the copies that have a price only, absent when none has one.
DROP MATERIALIZED VIEW mv_card_prices;

ALTER TABLE collection_entry
    ALTER COLUMN purchase_price DROP NOT NULL;
UPDATE collection_entry
SET purchase_price = NULL
WHERE purchase_price = 0;

CREATE MATERIALIZED VIEW mv_card_prices AS
WITH aggregated_entry AS (
    SELECT collection_entry.set_code,
           collection_entry.collector_number,
           collection_entry.language_code,
           collection_entry.foil,
           collection_entry.user_id,
           LEAST(sum(collection_entry.quantity), 255::bigint)::integer                                         AS quantity,
           (sum(collection_entry.purchase_price::bigint * collection_entry.quantity)::bigint /
            NULLIF(sum(collection_entry.quantity) FILTER (WHERE collection_entry.purchase_price IS NOT NULL),
                   0))::integer                                                                                AS purchase_price,
           min(collection_entry.added_at)                                                                      AS added_at
    FROM collection_entry
    GROUP BY collection_entry.set_code, collection_entry.collector_number, collection_entry.language_code,
             collection_entry.foil, collection_entry.user_id
)
SELECT c.set_code,
       c.collector_number,
       c.language_code,
       ce.foil,
       c.name,
       c.rarity,
       c.scryfall_id,
       ce.user_id,
       ce.quantity,
       ce.purchase_price,
       ce.added_at,
       lcp.low,
       lcp.trend,
       lcp.avg
FROM card c
         JOIN aggregated_entry ce
              ON c.set_code::text = ce.set_code::text AND c.collector_number::text = ce.collector_number::text AND
                 c.language_code::text = ce.language_code::text
         LEFT JOIN mv_last_cardmarket_prices lcp
                   ON c.set_code = lcp.set_code AND c.collector_number = lcp.collector_number AND ce.foil = lcp.foil;

CREATE UNIQUE INDEX mv_card_prices_unique
    ON mv_card_prices (set_code, collector_number, language_code, foil, user_id);

CREATE INDEX idx_mv_card_prices_name_trgm
    ON mv_card_prices USING GIN (name gin_trgm_ops);
