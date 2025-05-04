-- Add migration script here
ALTER TABLE tg_group_joined ADD COLUMN ckb_address VARCHAR(255);
ALTER TABLE tg_group_joined ADD COLUMN dob DATE;

CREATE TABLE IF NOT EXISTS tg_group_admins (
    chat_id VARCHAR(255), 
    user_id BIGINT, 
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (chat_id, user_id)
);