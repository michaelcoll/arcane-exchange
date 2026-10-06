-- ADR-0017 : les images de cartes sont téléchargées et stockées par la plateforme.
-- `image_source` est la source retenue ; NULL signifie que la carte est en attente (jamais en
-- erreur). Toutes les cartes existantes partent en attente, ce qui porte le backfill.
-- `image_has_back` indique qu'un verso a été enregistré avec le recto.
ALTER TABLE card
    ADD COLUMN image_source   VARCHAR(32),
    ADD COLUMN image_has_back BOOLEAN NOT NULL DEFAULT FALSE,
    ADD CONSTRAINT card_image_source_check
        CHECK (image_source IN ('gatherer_localized', 'gatherer_en', 'scryfall')),
    ADD CONSTRAINT card_image_back_needs_source_check
        CHECK (image_source IS NOT NULL OR NOT image_has_back);
