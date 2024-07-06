-- SQL commands to upgrade
-- Revision: f44e620f-60e0-4470-8904-44b4022b11a5
-- Down Revision: None
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) NOT NULL,
    password VARCHAR(255) NOT NULL
);