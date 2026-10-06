ALTER TABLE trade
    ADD COLUMN initiator_confirmed_at TIMESTAMPTZ,
    ADD COLUMN respondent_confirmed_at TIMESTAMPTZ,
    ADD COLUMN initiator_rating SMALLINT CHECK (initiator_rating BETWEEN 0 AND 5),
    ADD COLUMN respondent_rating SMALLINT CHECK (respondent_rating BETWEEN 0 AND 5);
