-- A Purchase is a dated buy of a player, independent of the collection entries it brought
-- (ADR-0023): its content has no reference to `collection_entry`, so a Trade, a move, a removal or
-- an import never changes it. Deleting a purchase deletes its content, never a collection entry.
CREATE TABLE purchase
(
    id           UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id      VARCHAR(50) NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind         VARCHAR(20) NOT NULL,
    amount_cents INTEGER     NOT NULL,
    purchased_on DATE        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Sealed product only.
    product_type VARCHAR(30),
    set_code     VARCHAR(6) REFERENCES set_name (set_code),
    name         TEXT,
    CONSTRAINT purchase_kind_check CHECK (kind IN ('sealed_product', 'single_card')),
    CONSTRAINT purchase_amount_positive CHECK (amount_cents > 0),
    CONSTRAINT purchase_sealed_product_check CHECK (
        (kind = 'sealed_product' AND product_type IS NOT NULL AND set_code IS NOT NULL)
            OR (kind = 'single_card' AND product_type IS NULL AND set_code IS NULL AND name IS NULL)),
    CONSTRAINT purchase_product_type_check CHECK (
        product_type IN ('BOOSTER', 'DISPLAY', 'BUNDLE', 'PRERELEASE_PACK', 'PRECONSTRUCTED_DECK')),
    -- An absent name is NULL, never an empty or blank string.
    CONSTRAINT purchase_name_trimmed CHECK (name IS NULL OR (name = btrim(name) AND name <> ''))
);

CREATE INDEX purchase_user_date_idx ON purchase (user_id, purchased_on DESC, created_at DESC);

-- What a purchase contained: one line per card (set, number, language) and finish.
CREATE TABLE purchase_content
(
    purchase_id      UUID        NOT NULL REFERENCES purchase (id) ON DELETE CASCADE,
    set_code         VARCHAR(6)  NOT NULL,
    collector_number VARCHAR(10) NOT NULL,
    language_code    VARCHAR(2)  NOT NULL,
    foil             BOOLEAN     NOT NULL,
    quantity         INTEGER     NOT NULL CHECK (quantity > 0),
    PRIMARY KEY (purchase_id, set_code, collector_number, language_code, foil),
    FOREIGN KEY (set_code, collector_number, language_code) REFERENCES card (set_code, collector_number, language_code)
);
