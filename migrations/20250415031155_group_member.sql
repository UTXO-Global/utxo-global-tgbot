-- Add migration script here

ALTER TABLE tg_group_joined ADD COLUMN balances TEXT DEFAULT '{}';