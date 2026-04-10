-- Example issues for migrate-doctor MVP
CREATE INDEX idx_users_email ON users (email);

ALTER TABLE users ADD COLUMN legacy_id text DEFAULT '';

ALTER TABLE orders ADD CONSTRAINT fk_orders_user FOREIGN KEY (user_id) REFERENCES users (id);

DROP TABLE old_import_staging;
