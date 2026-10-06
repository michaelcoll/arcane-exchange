-- 1. Contrôle de divergence : deux variantes d'une même carte (set_code, collector_number,
-- language_code) doivent partager la même définition. Une valeur non nulle face à un NULL sur
-- cardmarket_id ou the_gatherer_id est un état normal (les workers Cardmarket/Gatherer résolvent
-- chaque variante foil indépendamment) ; deux valeurs non nulles différentes, ou une divergence
-- sur name/rarity/scryfall_id, est une véritable anomalie qui doit interrompre la migration.
DO
$$
    DECLARE
        divergent_keys TEXT;
    BEGIN
        SELECT string_agg(format('(%s, %s, %s)', set_code, collector_number, language_code), ', ')
        INTO divergent_keys
        FROM (SELECT set_code, collector_number, language_code
              FROM card
              GROUP BY set_code, collector_number, language_code
              HAVING count(DISTINCT name) > 1
                  OR count(DISTINCT rarity) > 1
                  OR count(DISTINCT scryfall_id) > 1
                  OR count(DISTINCT cardmarket_id) FILTER (WHERE cardmarket_id IS NOT NULL) > 1
                  OR count(DISTINCT the_gatherer_id) FILTER (WHERE the_gatherer_id IS NOT NULL) > 1) AS d;

        IF divergent_keys IS NOT NULL THEN
            RAISE EXCEPTION 'card: variants disagree on identity for keys %; cannot merge foil/non-foil rows', divergent_keys;
        END IF;
    END
$$;

-- 2. Supprimer les objets dépendants de card.foil, dans l'ordre de leurs dépendances
-- (mv_card_prices dépend de mv_last_cardmarket_prices).
DROP MATERIALIZED VIEW mv_card_prices;
DROP MATERIALIZED VIEW mv_last_cardmarket_prices;
DROP VIEW v_tradable_entry;

-- 3. Supprimer les clés étrangères vers card, qui portent foil.
ALTER TABLE collection_entry
    DROP CONSTRAINT collection_entry_card_fk;
ALTER TABLE trade_card
    DROP CONSTRAINT trade_card_card_fk;

-- 4. Fusionner les variantes : reporter sur la ligne survivante les valeurs non nulles de
-- cardmarket_id et the_gatherer_id venues de l'autre variante, puis ne garder qu'une ligne par
-- (set_code, collector_number, language_code).
WITH merged AS (SELECT set_code,
                        collector_number,
                        language_code,
                        max(cardmarket_id)   AS cardmarket_id,
                        max(the_gatherer_id) AS the_gatherer_id,
                        min(foil::int)       AS surviving_foil
                 FROM card
                 GROUP BY set_code, collector_number, language_code)
UPDATE card c
SET cardmarket_id   = merged.cardmarket_id,
    the_gatherer_id = merged.the_gatherer_id
FROM merged
WHERE c.set_code = merged.set_code
  AND c.collector_number = merged.collector_number
  AND c.language_code = merged.language_code
  AND c.foil = (merged.surviving_foil::boolean);

DELETE
FROM card c
    USING (SELECT set_code, collector_number, language_code, min(foil::int) AS surviving_foil
           FROM card
           GROUP BY set_code, collector_number, language_code) AS keep
WHERE c.set_code = keep.set_code
  AND c.collector_number = keep.collector_number
  AND c.language_code = keep.language_code
  AND c.foil <> (keep.surviving_foil::boolean);

-- 5. Retirer foil de card : la clé primaire devient (set_code, collector_number, language_code).
ALTER TABLE card
    DROP CONSTRAINT card_pk;
ALTER TABLE card
    DROP COLUMN foil;
ALTER TABLE card
    ADD CONSTRAINT card_pk PRIMARY KEY (set_code, collector_number, language_code);

-- 6. Recréer les clés étrangères sur trois colonnes.
ALTER TABLE collection_entry
    ADD CONSTRAINT collection_entry_card_fk FOREIGN KEY (set_code, collector_number, language_code)
        REFERENCES card (set_code, collector_number, language_code);
ALTER TABLE trade_card
    ADD CONSTRAINT trade_card_card_fk FOREIGN KEY (set_code, collector_number, language_code)
        REFERENCES card (set_code, collector_number, language_code);

-- 7. Recréer les trois vues, en réorientant la finition sur l'exemplaire.

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
                 c.language_code::text = ce.language_code::text
         LEFT JOIN mv_last_cardmarket_prices lcp
                   ON c.set_code = lcp.set_code AND c.collector_number = lcp.collector_number AND ce.foil = lcp.foil;

CREATE UNIQUE INDEX mv_card_prices_unique
    ON mv_card_prices (set_code, collector_number, language_code, foil, user_id);

CREATE INDEX idx_mv_card_prices_name_trgm
    ON mv_card_prices USING GIN (name gin_trgm_ops);
