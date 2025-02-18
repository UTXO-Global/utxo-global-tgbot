-- Add migration script here

CREATE TABLE IF NOT EXISTS tg_groups (
    chat_id VARCHAR(255), 
    name VARCHAR(255),
    status SMALLINT NOT NULL DEFAULT 1,
    token_address VARCHAR(255) DEFAULT NULL,
    min_approve_balance BIGINT DEFAULT 0,
    min_approve_age INTEGER DEFAULT 18,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (chat_id)
);

CREATE TABLE IF NOT EXISTS tg_group_joined (
    chat_id VARCHAR(255), 
    user_id BIGINT,
    user_name VARCHAR(255),
    status SMALLINT NOT NULL DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (chat_id, user_id)
);