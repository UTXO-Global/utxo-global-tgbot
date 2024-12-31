from db import get_db_connection
import config

conn = get_db_connection()
cur = conn.cursor()

# Create Table
cur.execute('''
            CREATE TABLE members (
                tgid BIGINT PRIMARY KEY, 
                tgname VARCHAR,
                status SMALLINT NOT NULL DEFAULT 0,
                ckb_address VARCHAR,
                balance NUMERIC,
                dob DATE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            ''')

conn.commit()

config.logger.info(f"Init db success")
cur.close()
conn.close()