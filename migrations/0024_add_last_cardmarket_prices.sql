CREATE MATERIALIZED VIEW mv_last_cardmarket_prices AS
WITH last_price AS (SELECT id_produit, MAX(date) AS last_date
                    FROM cardmarket_price
                    GROUP BY id_produit)
SELECT DISTINCT c.set_code,
                c.collector_number,
                c.foil,
                CASE WHEN c.foil THEN cmp.low_foil ELSE cmp.low END     AS low,
                CASE WHEN c.foil THEN cmp.trend_foil ELSE cmp.trend END AS trend,
                CASE WHEN c.foil THEN cmp.avg_foil ELSE cmp.avg END     AS avg
FROM card c
         LEFT JOIN last_price lp ON c.cardmarket_id = lp.id_produit
         LEFT JOIN cardmarket_price cmp ON c.cardmarket_id = cmp.id_produit AND cmp.date = lp.last_date;

CREATE UNIQUE INDEX mv_last_cardmarket_prices_unique ON mv_last_cardmarket_prices (set_code, collector_number, foil);
