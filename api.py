from flask import Flask, abort, jsonify, request, Response, json
from llama import DeepthoughtModel, chat_content
from db import psql_db
import os
import config
import uuid
from flask_restx import Api, Resource, fields, reqparse

app = Flask(__name__)
api = Api(
    app,
    version='1.0',
    title='Coti Agent Swagger API',
    description='This is an automatically generated Swagger UI for all APIs',
    doc='/api-docs/'
)

@app.errorhandler(501)
def not_implemented_error(e):
    return jsonify({
        "error": "Not Implemented",
        "message": "This feature is not implemented yet."
    }), 501

@app.errorhandler(500)
def not_internal_server_error(e):
    return jsonify({
        "error": "Internal server error",
        "message": "Something wrong!"
    }), 500

# Namespace /v2
nsv2 = api.namespace('v2', description='V2 APIs')
@nsv2.doc(
    params={
        'x-app-key': {
            'in': 'header',
            'type': 'string',
            'required': True,
            'description': 'The authorization x-app-key header (required)'
        }
    },
    responses={
        400: '''
            {"error": "Missing x-app-key"}
        ''',
        403: '''
            {"error": "Invalid x-app-key"}
        '''
    }
)
class V2Middleware(Resource):
    def dispatch_request(self, *args, **kwargs):
        app_key = request.headers.get('x-app-key')
        if not request.headers.get('x-app-key'):
            return Response(json.dumps({"error": "Missing x-app-key"}), 400) 
        if app_key != os.environ['APP_KEY']:
            return Response(json.dumps({ "error": "Invalid x-app-key" }), 403)
        return super().dispatch_request(*args, **kwargs)
    
@nsv2.route('/instructions')
class Instruction(V2Middleware):
    @nsv2.param('token_address', 'The token address (required)', type=str)
    def get(self):
        token_address = request.args.get('token_address')
        instructions = psql_db.get_agent_instructions(token_address)
        return jsonify(instructions)
    
    create_instruction_model = nsv2.model('CreateInstructionModel', {
        'token_address': fields.String(required=True, description='The token address (required)'),
        'instruction': fields.String(required=True, description='The instruction content (required)'),
        'owner_address': fields.String(required=True, description='The creator address (required)'),
    })
    @nsv2.expect(create_instruction_model)
    def post(self):
        json_data = request.get_json(force=True)
        token_address = json_data['token_address']
        instruction = json_data['instruction']
        owner_address = json_data['owner_address']
        
        psql_db.insert_agent_instruction(token_address, owner_address, instruction)
        return jsonify(success=True)
    
    update_instruction_model = nsv2.model('UpdateInstructionModel', {
        'instruction_id': fields.Integer(required=True, description='The instruction Id (required)'),
        'instruction': fields.String(required=True, description='The instruction content (required)'),
    })
    @nsv2.expect(update_instruction_model)
    def patch(self):
        json_data = request.get_json(force=True)
        instruction_id = json_data['instruction_id']
        instruction = json_data['instruction']
        psql_db.update_agent_instruction(instruction_id, instruction)
        return jsonify(success=True)
    
    delete_instruction_model = nsv2.model('DeleteInstructionModel', {
        'instruction_id': fields.Integer(required=True, description='The instruction Id (required)'),
    })
    @nsv2.expect(delete_instruction_model)
    def delete(self):
        json_data = request.get_json(force=True)
        instruction_id = json_data['instruction_id']
        psql_db.delete_agent_instruction(instruction_id)
        return jsonify(success=True)

# BotChat
@nsv2.route('/chat')
@nsv2.param('token_address', 'The token address (required)', type=str)
@nsv2.param('user_address', 'The user address (required)', type=str)
class BotChat(V2Middleware):
    # Define the query parameters and headers
    query_parser = reqparse.RequestParser()
    query_parser.add_argument(
        'token_address', type=str, location='args', required=True, help='The token address (required)'
    )
    query_parser.add_argument(
        'user_address', type=str, location='args', required=True, help='The user address (required)'
    )
    query_parser.add_argument(
        'x-app-key', type=str, location='headers', required=True, help='The authorization x-app-key header (required)'
    )

    # Define the expected JSON body for POST
    bot_chat_model = nsv2.model('BotChatBody', {
        'msg': fields.String(required=True, description='The message content'),
    })

    def parse_data(self):
        # Parse query parameters and headers
        args = self.query_parser.parse_args()
        self.token_address = args['token_address']
        self.user_address = args['user_address']
        self.agent_messages = psql_db.get_agent_messages(self.token_address, self.user_address)

    @nsv2.expect(query_parser, bot_chat_model)
    @nsv2.doc(
        responses={
            200: '''{"response": "Hello"}'''
        }
    )
    def post(self):
        self.parse_data()
        json_data = request.get_json(force=True)
        user_msg = json_data['msg']

        agent_instructions = psql_db.get_agent_instructions(self.token_address)
        context = list(map(chat_content, self.agent_messages))
        print(agent_instructions)
        context.insert(0, {"role": "system", "content": "\n".join(list(map(lambda x: x['content'], agent_instructions)))})

        chatbot = DeepthoughtModel(context)
        agent_reply = chatbot.chat(user_msg)
        psql_db.insert_agent_message(self.token_address, self.user_address, user_msg, agent_reply)
        return jsonify(response=agent_reply)
    
    @nsv2.expect(query_parser)
    @nsv2.doc(
        responses={
            200: '''
                {
                    "messages": [{
                        "content": "what different between dog and cat",
                        "role": "user"
                    }]
                }
                '''
        }
    )
    def get(self):
        self.parse_data()
        return jsonify(messages=list(map(chat_content, self.agent_messages)))


# Old Api
@app.route('/new-agent', methods=['POST'])
def new_agentv1():
    json_data = request.get_json(force=True)
    topic = json_data['topic']
    token_address = "v1_" + str(uuid.uuid4())
    psql_db.insert_agent_instruction(token_address, "", topic)
    return jsonify(agent_id=token_address)

@app.route('/chat', methods=['POST', 'GET'])
def chatv1():
    agent_id = request.args.get('agent_id')

    if request.method == 'POST':
        if agent_id is None:
            context = []
            chatbot = DeepthoughtModel(context)
            json_data = request.get_json(force=True)
            user_msg = json_data['msg']
            agent_reply = chatbot.chat(user_msg)['content']
            return jsonify(response=agent_reply)
        
        json_data = request.get_json(force=True)
        user_msg = json_data['msg']

        agent_topic = psql_db.get_agent_instructions(agent_id)
        agent_messages = psql_db.get_agent_messages(agent_id)
        context = [
            {"role": "system", "content": agent_topic},
        ]
        context_msg = list(map(chat_content, agent_messages))
        context.extend(context_msg)

        chatbot = DeepthoughtModel(context)
        agent_reply = chatbot.chat(user_msg)['content']
        psql_db.insert_agent_message(agent_id, user_msg, agent_reply)
        return jsonify(response=agent_reply)
    
    if request.method == "GET":
        if not agent_id:
            abort(404)
        agent_messages = psql_db.get_agent_messages(agent_id)
        return jsonify(messages=list(map(chat_content, agent_messages)))

if __name__ == '__main__':
    app.run(debug=True, host="0.0.0.0", port=8080)