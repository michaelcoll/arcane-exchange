CREATE TABLE trade
(
    id                    UUID PRIMARY KEY,
    initiator_user_id     VARCHAR(50) NOT NULL,
    respondent_user_id    VARCHAR(50) NOT NULL,
    status                VARCHAR(20) NOT NULL,
    initiator_amount_due  INTEGER,
    respondent_amount_due INTEGER,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT trade_initiator_fk FOREIGN KEY (initiator_user_id) REFERENCES users (id),
    CONSTRAINT trade_respondent_fk FOREIGN KEY (respondent_user_id) REFERENCES users (id)
);

CREATE TABLE trade_card
(
    trade_id         UUID        NOT NULL,
    set_code         VARCHAR(5)  NOT NULL,
    collector_number VARCHAR(10) NOT NULL,
    language_code    VARCHAR(2)  NOT NULL,
    foil             BOOLEAN     NOT NULL,
    owner_user_id    VARCHAR(50) NOT NULL,
    quantity         INTEGER     NOT NULL,

    CONSTRAINT trade_card_pk PRIMARY KEY (trade_id, set_code, collector_number, language_code, foil, owner_user_id),
    CONSTRAINT trade_card_trade_fk FOREIGN KEY (trade_id) REFERENCES trade (id),
    CONSTRAINT trade_card_card_fk FOREIGN KEY (set_code, collector_number, language_code, foil)
        REFERENCES card (set_code, collector_number, language_code, foil),
    CONSTRAINT trade_card_owner_fk FOREIGN KEY (owner_user_id) REFERENCES users (id)
);
