-- Qui a abandonné un trade, en colonnes appariées par partie comme les acceptations, confirmations
-- et notes (ADR-0005). Les deux restent NULL pour un abandon automatique (cartes réservées par un
-- autre trade) et pour les trades déjà abandonnés : ils n'ont pas d'auteur.
ALTER TABLE trade
    ADD COLUMN initiator_abandoned_at  TIMESTAMPTZ,
    ADD COLUMN respondent_abandoned_at TIMESTAMPTZ;
