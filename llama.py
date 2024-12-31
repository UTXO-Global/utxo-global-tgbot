import ollama
import config
import os

role_enum = {
    0: 'user',
    1: 'assistant'
}

def chat_content(msg_row: list[str]):
    role = role_enum[msg_row[0]]
    return {"role": role, "content": msg_row[1]}

class DeepthoughtModel:
    def __init__(self, messages: list[dict[str, str]]):
        self.messages = messages
    
    def chat(self, msg: str):
        self.messages.append({"role": "user", "content": msg})
        response = ollama.chat(
            model=os.environ['MODEL_URL'],
            messages=self.messages
        )
        return response.message.content

