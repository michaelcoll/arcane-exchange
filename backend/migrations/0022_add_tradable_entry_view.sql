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
                                JOIN card c ON (c.set_code, c.collector_number, c.language_code, c.foil) =
                                               (ce.set_code, ce.collector_number, ce.language_code, ce.foil)
                                LEFT JOIN trading_binders tb
                                          ON tb.user_id = ce.user_id AND tb.binder_name = ce.binder_name
                       WHERE u.visibility <> 'private'
                         AND (u.visibility = 'public' OR tb.binder_name IS NOT NULL)),
     -- `kept_copies` is deducted per `collection_entry` row (i.e. per binder), matching
     -- `collection_rarity_filters_repository_adapter::list_with_counts` (spec 020's "Proposés"
     -- counter) row for row, so the two stay numerically consistent for a `trade` user. Summing
     -- the raw quantity first and deducting once per card (as `public` does) would let a card
     -- split across several checked binders offer more copies than the profile screen shows.
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
