-- Add migration script here

ALTER TABLE tg_group_joined ADD COLUMN expired TIMESTAMP DEFAULT CURRENT_TIMESTAMP;