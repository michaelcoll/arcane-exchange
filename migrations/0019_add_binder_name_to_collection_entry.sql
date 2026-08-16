ALTER TABLE collection_entry
    ADD COLUMN binder_name VARCHAR(255);

ALTER TABLE collection_entry
    DROP CONSTRAINT collection_entry_pk;

ALTER TABLE collection_entry
    ADD CONSTRAINT collection_entry_uk
        UNIQUE NULLS NOT DISTINCT (set_code, collector_number, language_code, foil, user_id, binder_name);

DROP MATERIALIZED VIEW IF EXISTS mv_card_prices;

CREATE MATERIALIZED VIEW mv_card_prices AS
WITH last_price AS (SELECT id_produit, MAX(date) AS last_date
                    FROM cardmarket_price
                    GROUP BY id_produit),
     aggregated_entry AS (SELECT set_code,
                                  collector_number,
                                  language_code,
                                  foil,
                                  user_id,
                                  LEAST(SUM(quantity), 255)::INTEGER AS quantity,
                                  -- SUM(bigint) returns numeric in Postgres; cast back to bigint before
                                  -- dividing so the average truncates like the import parser's merge does
                                  -- (total_cost / new_qty on Rust integers), instead of rounding.
                                  COALESCE(SUM(purchase_price::BIGINT * quantity)::BIGINT / NULLIF(SUM(quantity), 0),
                                           0)::INTEGER                AS purchase_price,
                                  MIN(added_at)                       AS added_at
                           FROM collection_entry
                           GROUP BY set_code, collector_number, language_code, foil, user_id)
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
       CASE WHEN c.foil THEN cmp.low_foil ELSE cmp.low END     AS low,
       CASE WHEN c.foil THEN cmp.trend_foil ELSE cmp.trend END AS trend,
       CASE WHEN c.foil THEN cmp.avg_foil ELSE cmp.avg END     AS avg
FROM card c
         JOIN aggregated_entry ce ON c.set_code = ce.set_code
    AND c.collector_number = ce.collector_number
    AND c.language_code = ce.language_code
    AND c.foil = ce.foil
         LEFT JOIN last_price lp ON c.cardmarket_id = lp.id_produit
         LEFT JOIN cardmarket_price cmp ON c.cardmarket_id = cmp.id_produit
    AND cmp.date = lp.last_date;

CREATE UNIQUE INDEX mv_card_prices_unique ON mv_card_prices (set_code, collector_number, language_code, foil, user_id);
