-- ADR-0004 : un seul trade actif par paire de joueurs, quel que soit le sens.
-- Jusqu'ici garanti par un verrou en mémoire du process ; désormais porté par la donnée.

-- 1. Nettoyage préalable : une course de création a pu laisser plusieurs trades actifs pour une
-- même paire. On garde le plus avancé (pour ne pas abandonner un accord déjà conclu), puis le plus
-- ancien à égalité, et on abandonne les autres, sans quoi l'index unique ci-dessous ne pourrait
-- pas être créé.
UPDATE trade
SET status     = 'ABANDONED',
    updated_at = NOW()
WHERE id IN (SELECT id
             FROM (SELECT id,
                          ROW_NUMBER() OVER (
                              PARTITION BY LEAST(initiator_user_id, respondent_user_id),
                                  GREATEST(initiator_user_id, respondent_user_id)
                              ORDER BY CASE status
                                           WHEN 'FULLY_ACCEPTED' THEN 0
                                           WHEN 'ONE_ACCEPTED' THEN 1
                                           ELSE 2 END,
                                  created_at, id) AS rank
                   FROM trade
                   WHERE status IN ('PENDING', 'ONE_ACCEPTED', 'FULLY_ACCEPTED')) AS active
             WHERE rank > 1);

-- 2. L'invariant lui-même.
CREATE UNIQUE INDEX trade_one_active_per_pair
    ON trade (LEAST(initiator_user_id, respondent_user_id), GREATEST(initiator_user_id, respondent_user_id))
    WHERE status IN ('PENDING', 'ONE_ACCEPTED', 'FULLY_ACCEPTED');
