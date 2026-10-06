CREATE INDEX trade_card_card_owner_idx
    ON trade_card (set_code, collector_number, language_code, foil, owner_user_id);
