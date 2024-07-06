-- SQL commands to upgrade
-- Revision: 622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd
-- Down Revision: f44e620f-60e0-4470-8904-44b4022b11a5
CREATE TABLE IF NOT EXISTS wallets (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    address VARCHAR(255) NOT NULL,
    encrypted_seed_phrase TEXT NOT NULL,
    encrypted_private_key TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users (id)
);