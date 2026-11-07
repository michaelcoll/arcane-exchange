-- Le catalogue : toutes les impressions de cartes connues de Scryfall (hors tokens), clé par set et
-- numéro, sans langue ni finition. Indépendant de la possession : aucune clé étrangère vers `card`,
-- ni l'inverse, pour pouvoir le recharger sans toucher aux Cards. Remplacé en entier à chaque
-- chargement.
CREATE TABLE catalog_card
(
    set_code         VARCHAR(5)   NOT NULL,
    collector_number VARCHAR(10)  NOT NULL,
    rarity           VARCHAR(1)   NOT NULL,
    name             VARCHAR(255) NOT NULL,
    french_name      VARCHAR(255),
    scryfall_id      UUID         NOT NULL,
    cardmarket_id    INTEGER,

    CONSTRAINT catalog_card_pk PRIMARY KEY (set_code, collector_number),
    CONSTRAINT catalog_card_set_name_fk FOREIGN KEY (set_code) REFERENCES set_name (set_code)
);
