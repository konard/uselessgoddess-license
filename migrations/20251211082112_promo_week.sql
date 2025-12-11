CREATE TABLE IF NOT EXISTS claimed_promos (
    tg_user_id INTEGER NOT NULL,
    promo_name TEXT NOT NULL,
    claimed_at DATETIME NOT NULL,
    PRIMARY KEY (tg_user_id, promo_name)
);