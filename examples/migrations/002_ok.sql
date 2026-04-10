CREATE INDEX CONCURRENTLY idx_users_name ON users (name);

ALTER TABLE users ADD COLUMN phone text;
