import psycopg2
import os
import config

def get_db_connection():
    conn = psycopg2.connect(
        host="localhost",
        database="coti_agent",
        user=os.environ['DB_USERNAME'],
        password=os.environ['DB_PASSWORD'])
    return conn

class PsqlDb:
    def __init__(self):
        self.conn = get_db_connection()
    
    def insert_agent_instruction(self, token_address: str, owner_address: str, instruction: str):
        cur = self.conn.cursor()
        cur.execute('INSERT INTO agents (token_address, owner_address) VALUES (lower(%s), lower(%s)) ON CONFLICT (token_address) DO NOTHING;', (token_address,owner_address))
        cur.execute('INSERT INTO agent_instructions (token_address, content) VALUES (lower(%s), %s);', (token_address,instruction))
        self.conn.commit()
        cur.close()
    
    def get_agent_instructions(self, token_address: str):
        cur = self.conn.cursor()
        cur.execute('SELECT id, content FROM agent_instructions WHERE token_address = lower(%s);', (token_address,))
        column_names = [desc[0] for desc in cur.description]
        agent_instructions = cur.fetchall()
        cur.close()
        return [dict(zip(column_names, row)) for row in agent_instructions]

    def insert_agent_message(self, token_address: str, user_address: str, user_msg: str, assistant_msg: str):
        cur = self.conn.cursor()
        cur.executemany('''INSERT INTO agent_messages (token_address, user_address, role, content) VALUES (lower(%s), lower(%s), %s, %s)''', [(token_address, user_address, 0, user_msg), (token_address, user_address, 1, assistant_msg)])
        self.conn.commit()
        cur.close()

    def get_agent_messages(self, token_address: str, user_address: str):
        cur = self.conn.cursor()
        cur.execute('SELECT role, content, created_at FROM agent_messages WHERE token_address = lower(%s) AND user_address = lower(%s);', (token_address,user_address))
        agent_messages = cur.fetchall()
        cur.close()
        return agent_messages
    
    def update_agent_instruction(self, instruction_id: int, instruction: str):
        cur = self.conn.cursor()
        cur.execute('UPDATE agent_instructions SET content = %s WHERE id = %s;', (instruction, instruction_id))
        self.conn.commit()
        cur.close()

    def delete_agent_instruction(self, instruction_id: int):
        cur = self.conn.cursor()
        cur.execute('DELETE FROM agent_instructions WHERE id = %s;', (instruction_id))
        self.conn.commit()
        cur.close()

psql_db = PsqlDb()