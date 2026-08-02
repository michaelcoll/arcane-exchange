CREATE INDEX idx_users_username_trgm ON users USING GIN (LOWER(username) gin_trgm_ops);
