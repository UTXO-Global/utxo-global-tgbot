from db import get_db_connection
import config

conn = get_db_connection()
cur = conn.cursor()

# Create Table
cur.execute('''
            CREATE TABLE agents (
                token_address VARCHAR PRIMARY KEY,
                owner_address VARCHAR NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE agent_instructions (
                id SERIAL PRIMARY KEY,
                token_address VARCHAR NOT NULL,
                content TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE agent_messages (
                id SERIAL PRIMARY KEY,
                user_address VARCHAR NOT NULL,
                token_address VARCHAR NOT NULL,
                role SMALLINT NOT NULL,
                content TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX agent_messages_agent_id ON agent_messages (token_address);
            ''')

conn.commit()

cur.close()
conn.close()