-- The binder becomes an entity of its own, designated by an identifier, instead of a name copied
-- onto every `collection_entry` and `trading_binders` row (ADR-0022). A binder can then be empty,
-- and a rename keeps its trade opening.
CREATE TABLE binder
(
    id      UUID         NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id VARCHAR(50)  NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name    VARCHAR(255) NOT NULL,
    CONSTRAINT binder_user_name_uk UNIQUE (user_id, name),
    CONSTRAINT binder_name_trimmed CHECK (name = btrim(name) AND name <> '')
);

-- One binder per distinct name of each player, on the entries as on the trade openings.
INSERT INTO binder (user_id, name)
SELECT user_id, btrim(binder_name)
FROM collection_entry
WHERE btrim(binder_name) <> ''
UNION
SELECT user_id, btrim(binder_name)
FROM trading_binders
WHERE btrim(binder_name) <> '';

-- The tradable view reads the two columns about to be replaced: it is derived, so it is dropped
-- and recreated with the same output.
DROP VIEW v_tradable_entry;

ALTER TABLE collection_entry
    ADD COLUMN binder_id UUID REFERENCES binder (id);
UPDATE collection_entry ce
SET binder_id = b.id
FROM binder b
WHERE b.user_id = ce.user_id
  AND b.name = btrim(ce.binder_name);

ALTER TABLE collection_entry
    DROP CONSTRAINT collection_entry_uk;
ALTER TABLE collection_entry
    DROP COLUMN binder_name;

-- The old import never trimmed binder names, so two names that only differ by spaces ("A", "A ")
-- are now one binder, and a name made of spaces only is no binder at all. Entries of the same card,
-- finish, player and binder that collide this way become one, by the merge rule of the Collection
-- entries (ADR-0010): quantities add up (capped to the 255 an entry holds), the purchase price is
-- the average of the prices present weighted by quantity (a price of 0 is still the absent price
-- here, until the next migration), the added date is the earliest. The unique constraint, dropped
-- above, is added back once they are merged.
WITH colliding AS (
    DELETE FROM collection_entry ce
        WHERE EXISTS (SELECT 1
                      FROM collection_entry other
                      WHERE other.ctid <> ce.ctid
                        AND other.set_code = ce.set_code
                        AND other.collector_number = ce.collector_number
                        AND other.language_code = ce.language_code
                        AND other.foil = ce.foil
                        AND other.user_id = ce.user_id
                        AND other.binder_id IS NOT DISTINCT FROM ce.binder_id)
        RETURNING ce.*)
INSERT
INTO collection_entry (set_code, collector_number, language_code, foil, user_id, binder_id, quantity,
                       purchase_price, added_at)
SELECT set_code,
       collector_number,
       language_code,
       foil,
       user_id,
       binder_id,
       LEAST(SUM(quantity), 255),
       COALESCE((SUM(purchase_price::BIGINT * quantity) FILTER (WHERE purchase_price > 0) /
                 NULLIF(SUM(quantity) FILTER (WHERE purchase_price > 0), 0))::INTEGER, 0),
       MIN(added_at)
FROM colliding
GROUP BY set_code, collector_number, language_code, foil, user_id, binder_id;

ALTER TABLE collection_entry
    ADD CONSTRAINT collection_entry_uk
        UNIQUE NULLS NOT DISTINCT (set_code, collector_number, language_code, foil, user_id, binder_id);

ALTER TABLE trading_binders
    ADD COLUMN binder_id UUID REFERENCES binder (id) ON DELETE CASCADE;
UPDATE trading_binders tb
SET binder_id = b.id
FROM binder b
WHERE b.user_id = tb.user_id
  AND b.name = btrim(tb.binder_name);

-- An opening whose name is blank has no binder to be attached to: a binder needs a non-empty name,
-- so it cannot survive the migration and is dropped here, deliberately. The two names of a pair
-- like "A" and "A " now designate the same binder: one opening is kept.
DELETE
FROM trading_binders
WHERE binder_id IS NULL;
DELETE
FROM trading_binders keep_one
    USING trading_binders other
WHERE keep_one.binder_id = other.binder_id
  AND keep_one.ctid > other.ctid;
ALTER TABLE trading_binders
    DROP CONSTRAINT trading_binders_pkey;
ALTER TABLE trading_binders
    DROP COLUMN binder_name;
ALTER TABLE trading_binders
    DROP COLUMN user_id;
ALTER TABLE trading_binders
    ALTER COLUMN binder_id SET NOT NULL;
ALTER TABLE trading_binders
    ADD PRIMARY KEY (binder_id);

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
                                LEFT JOIN trading_binders tb ON tb.binder_id = ce.binder_id
                       WHERE u.visibility <> 'private'
                         AND (u.visibility = 'public' OR tb.binder_id IS NOT NULL)),
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
