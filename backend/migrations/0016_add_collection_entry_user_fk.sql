ALTER TABLE collection_entry
    ADD CONSTRAINT collection_entry_user_fk FOREIGN KEY (user_id) REFERENCES users (id);
