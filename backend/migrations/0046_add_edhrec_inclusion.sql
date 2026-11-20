-- ADR-0027 : le taux d'inclusion EDHREC est indexé par nom anglais de carte, pas par Card. On
-- stocke les deux nombres bruts ; le pourcentage est calculé à la lecture. Un nom absent
-- d'EDHREC n'a pas de ligne.
CREATE TABLE edhrec_inclusion
(
    card_name       VARCHAR(255) NOT NULL,
    num_decks       INTEGER      NOT NULL,
    potential_decks INTEGER      NOT NULL,
    fetched_at      TIMESTAMPTZ  NOT NULL,

    CONSTRAINT edhrec_inclusion_pk PRIMARY KEY (card_name)
);
