-- The binder where the cards a player receives by Transfer arrive, chosen in the player's trade
-- settings. Null (the default) means the binder named `trades`, created on the first reception.
-- Deleting the binder, including an import dropping it, resets the choice; a renaming keeps it.
ALTER TABLE users
    ADD COLUMN receiving_binder_id UUID REFERENCES binder (id) ON DELETE SET NULL;
