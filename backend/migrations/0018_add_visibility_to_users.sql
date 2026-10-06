ALTER TABLE users
    ADD COLUMN visibility VARCHAR(10) NOT NULL DEFAULT 'private'
        CHECK (visibility IN ('public', 'trade', 'private'));
