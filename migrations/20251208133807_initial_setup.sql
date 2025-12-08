-- Add migration script here
CREATE TABLE IF NOT EXISTS licenses (
    key TEXT PRIMARY KEY NOT NULL,
    tg_user_id INTEGER NOT NULL,
    expires_at DATETIME NOT NULL,
    is_blocked BOOLEAN NOT NULL DEFAULT FALSE
);
