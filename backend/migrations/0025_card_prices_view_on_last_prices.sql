DROP MATERIALIZED VIEW mv_card_prices;

CREATE MATERIALIZED VIEW mv_card_prices AS
WITH aggregated_entry AS (
    SELECT collection_entry.set_code,
           collection_entry.collector_number,
           collection_entry.language_code,
           collection_entry.foil,
           collection_entry.user_id,
           LEAST(sum(collection_entry.quantity), 255::bigint)::integer                                         AS quantity,
           COALESCE(sum(collection_entry.purchase_price::bigint * collection_entry.quantity)::bigint /
                    NULLIF(sum(collection_entry.quantity), 0), 0::bigint)::integer                              AS purchase_price,
           min(collection_entry.added_at)                                                                      AS added_at
    FROM collection_entry
    GROUP BY collection_entry.set_code, collection_entry.collector_number, collection_entry.language_code,
             collection_entry.foil, collection_entry.user_id
)
SELECT c.set_code,
       c.collector_number,
       c.language_code,
       c.foil,
       c.name,
       c.rarity,
       c.scryfall_id,
       c.the_gatherer_id,
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
                 c.language_code::text = ce.language_code::text AND c.foil = ce.foil
         LEFT JOIN mv_last_cardmarket_prices lcp
                   ON c.set_code = lcp.set_code AND c.collector_number = lcp.collector_number AND c.foil = lcp.foil;

CREATE UNIQUE INDEX mv_card_prices_unique
    ON mv_card_prices (set_code, collector_number, language_code, foil, user_id);

CREATE INDEX idx_mv_card_prices_name_trgm
    ON mv_card_prices USING GIN (name gin_trgm_ops);
