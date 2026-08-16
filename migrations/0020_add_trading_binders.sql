CREATE TABLE trading_binders
(
    user_id     VARCHAR(50)  NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    binder_name VARCHAR(255) NOT NULL,
    PRIMARY KEY (user_id, binder_name)
);
