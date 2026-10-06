DROP MATERIALIZED VIEW IF EXISTS mv_card_prices;

ALTER TABLE cardmarket_price
    DROP COLUMN avg1,
    DROP COLUMN avg7,
    DROP COLUMN avg30,
    DROP COLUMN avg1_foil,
    DROP COLUMN avg7_foil,
    DROP COLUMN avg30_foil;

ALTER TABLE collection_price_history
    DROP COLUMN avg1,
    DROP COLUMN avg7,
    DROP COLUMN avg30;

CREATE MATERIALIZED VIEW mv_card_prices AS
WITH last_price AS (SELECT id_produit, MAX(date) AS last_date
                    FROM cardmarket_price
                    GROUP BY id_produit)
SELECT c.set_code,
       c.collector_number,
       c.language_code,
       c.foil,
       c.name,
       c.rarity,
       c.scryfall_id,
       ce.user_id,
       ce.quantity,
       ce.purchase_price,
       CASE WHEN c.foil THEN cmp.low_foil ELSE cmp.low END     AS low,
       CASE WHEN c.foil THEN cmp.trend_foil ELSE cmp.trend END AS trend,
       CASE WHEN c.foil THEN cmp.avg_foil ELSE cmp.avg END     AS avg
FROM card c
         JOIN collection_entry ce ON c.set_code = ce.set_code
    AND c.collector_number = ce.collector_number
    AND c.language_code = ce.language_code
    AND c.foil = ce.foil
         LEFT JOIN last_price lp ON c.cardmarket_id = lp.id_produit
         LEFT JOIN cardmarket_price cmp ON c.cardmarket_id = cmp.id_produit
    AND cmp.date = lp.last_date;

CREATE UNIQUE INDEX mv_card_prices_unique ON mv_card_prices (set_code, collector_number, language_code, foil, user_id);
