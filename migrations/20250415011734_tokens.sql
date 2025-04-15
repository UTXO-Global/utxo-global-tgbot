-- Add migration script here

CREATE TABLE IF NOT EXISTS tokens (
    type_hash VARCHAR(255), 
    name VARCHAR(255),
    symbol VARCHAR(255),
    decimal VARCHAR(255),
    description VARCHAR(255),
    token_type SMALLINT NOT NULL DEFAULT 0,
    args VARCHAR(255),
    code_hash VARCHAR(255),
    hash_type VARCHAR(255),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (type_hash)
);