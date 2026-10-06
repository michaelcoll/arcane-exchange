ALTER TABLE trade
    ADD COLUMN initiator_accepted_at TIMESTAMPTZ,
    ADD COLUMN respondent_accepted_at TIMESTAMPTZ;
