CREATE TABLE collection_rarity_filters
(
    user_id     VARCHAR(50) NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    rarity      VARCHAR(1)  NOT NULL
        CHECK (rarity IN ('C', 'U', 'R', 'M', 'S')),
    is_open     BOOLEAN     NOT NULL DEFAULT FALSE,
    kept_copies SMALLINT    NOT NULL DEFAULT 0
        CHECK (kept_copies BETWEEN 0 AND 4),
    PRIMARY KEY (user_id, rarity)
);
