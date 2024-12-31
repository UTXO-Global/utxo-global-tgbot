import requests
import config
import os

def ask_bot(user_msg, bot_name, user_address):
    try:
        # Send query to backend
        response = requests.post(
            f"{os.environ['AGENT_URL']}/v2/chat?user_address={user_address}&token_address={bot_name}",
            json={
                'msg': user_msg
            },
            headers={
                'Content-Type': 'application/json',
                'app-key': os.environ['APP_KEY']
            }
        )
        response_data = response.json()

        if response.status_code == 200:
            return response_data.get('response')
        else:
            raise Exception("request failed")
    except Exception as e:
        print(f"Ask Bot failed {e}")
    return "Sorry, something went wrong."
