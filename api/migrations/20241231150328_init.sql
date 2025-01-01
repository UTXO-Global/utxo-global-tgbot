-- Add migration script here

CREATE TABLE members (
    tgid BIGINT PRIMARY KEY, 
    tgname VARCHAR,
    status SMALLINT NOT NULL DEFAULT 0,
    ckb_address VARCHAR,
    balance NUMERIC,
    dob DATE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);