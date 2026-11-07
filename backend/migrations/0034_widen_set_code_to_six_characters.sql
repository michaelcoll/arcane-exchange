-- Les codes de set Scryfall les plus longs font 6 caractères (`PMPS06` à `PMPS11`, promos Magic
-- Premiere Shop) : le chargement du catalogue les écartait. Toutes les colonnes `set_code` passent
-- de 5 à 6 caractères, comme la règle de `SetCode`.
--
-- Postgres refuse de changer le type d'une colonne lue par une vue : les trois vues sont supprimées
-- puis recréées à l'identique (définitions de 0027 et 0030). Elles sont dérivées, et la recréation
-- des vues matérialisées les remplit : aucune donnée n'est perdue.
DROP VIEW v_tradable_entry;
DROP MATERIALIZED VIEW mv_card_prices;
DROP MATERIALIZED VIEW mv_last_cardmarket_prices;

ALTER TABLE set_name
    ALTER COLUMN set_code TYPE VARCHAR(6);
ALTER TABLE card
    ALTER COLUMN set_code TYPE VARCHAR(6);
ALTER TABLE collection_entry
    ALTER COLUMN set_code TYPE VARCHAR(6);
ALTER TABLE trade_card
    ALTER COLUMN set_code TYPE VARCHAR(6);
ALTER TABLE catalog_card
    ALTER COLUMN set_code TYPE VARCHAR(6);

CREATE VIEW v_tradable_entry AS
WITH scoped_entry AS (SELECT ce.user_id,
                              ce.set_code,
                              ce.collector_number,
                              ce.language_code,
                              ce.foil,
                              ce.quantity,
                              u.visibility,
                              c.rarity
                       FROM collection_entry ce
                                JOIN users u ON u.id = ce.user_id
                                JOIN card c ON (c.set_code, c.collector_number, c.language_code) =
                                               (ce.set_code, ce.collector_number, ce.language_code)
                                LEFT JOIN trading_binders tb
                                          ON tb.user_id = ce.user_id AND tb.binder_name = ce.binder_name
                       WHERE u.visibility <> 'private'
                         AND (u.visibility = 'public' OR tb.binder_name IS NOT NULL)),
     row_proposed AS (SELECT se.user_id,
                              se.set_code,
                              se.collector_number,
                              se.language_code,
                              se.foil,
                              CASE
                                  WHEN se.visibility = 'public' THEN se.quantity
                                  WHEN COALESCE(f.is_open, FALSE)
                                      THEN GREATEST(se.quantity - COALESCE(f.kept_copies, 0), 0)
                                  ELSE 0
                                  END AS proposed_quantity
                       FROM scoped_entry se
                                LEFT JOIN collection_rarity_filters f ON (f.user_id, f.rarity) = (se.user_id, se.rarity))
SELECT user_id,
       set_code,
       collector_number,
       language_code,
       foil,
       LEAST(SUM(proposed_quantity), 255)::INTEGER AS proposed_quantity
FROM row_proposed
GROUP BY user_id, set_code, collector_number, language_code, foil
HAVING SUM(proposed_quantity) > 0;

CREATE MATERIALIZED VIEW mv_last_cardmarket_prices AS
WITH last_price AS (SELECT id_produit, MAX(date) AS last_date
                     FROM cardmarket_price
                     GROUP BY id_produit)
SELECT DISTINCT c.set_code,
                c.collector_number,
                f.foil,
                CASE WHEN f.foil THEN cmp.low_foil ELSE cmp.low END     AS low,
                CASE WHEN f.foil THEN cmp.trend_foil ELSE cmp.trend END AS trend,
                CASE WHEN f.foil THEN cmp.avg_foil ELSE cmp.avg END     AS avg
FROM card c
         CROSS JOIN (VALUES (FALSE), (TRUE)) AS f (foil)
         LEFT JOIN last_price lp ON c.cardmarket_id = lp.id_produit
         LEFT JOIN cardmarket_price cmp ON c.cardmarket_id = cmp.id_produit AND cmp.date = lp.last_date;

CREATE UNIQUE INDEX mv_last_cardmarket_prices_unique ON mv_last_cardmarket_prices (set_code, collector_number, foil);

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
