import psycopg2
import os
import config

def get_db_connection():
    conn = psycopg2.connect(
        host="localhost",
        database="utxo-global-tgbot",
        user=os.environ['DB_USERNAME'],
        password=os.environ['DB_PASSWORD'])
    return conn

class PsqlDb:
    def __init__(self):
        self.conn = get_db_connection()
    
    def insert_member(self, tgid: int, tgname: str):
        cur = self.conn.cursor()
        cur.execute('INSERT INTO members (tgid, tgname) VALUES (%s, %s) ON CONFLICT (tgid) DO NOTHING ;', (tgid, tgname))
        self.conn.commit()
        cur.close()

psql_db = PsqlDb()