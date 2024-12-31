from flask import Flask, abort, jsonify, request
from db import psql_db
from flask_restx import Api, Resource, fields
from telegram import Bot
import os
import config

tgbot = Bot(token=os.environ['TELEGRAM_TOKEN'])

app = Flask(__name__)
api = Api(
    app,
    version='1.0',
    title='CKB Agent Swagger API',
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

# Namespace /v1
nsv1 = api.namespace('v1', description='V1 APIs')

@nsv1.route('/verify')
class VerifyAddress(Resource):
    verify_address_model = nsv1.model('VerifyAddressModel', {
        'telegram': fields.String(required=True, description='Telegram account @_ (required)'),
        'ckb_address': fields.String(required=True, description='CKB Address (required)'),
        'signature': fields.String(required=True, description='Signature provided by sign telegram message using ckb_address account (required)'),
        'dob': fields.String(required=True, description='Day of Birth with this format YYYY/MM/DD (required)'),
    })
    @nsv1.expect(verify_address_model)
    def post(self):
        json_data = request.get_json(force=True)
        telegram = json_data['telegram']
        ckb_address = json_data['ckb_address']
        signature = json_data['signature']
        dob = json_data['dob']

        # TODO: Verify signed message
        message = f"My tg: {telegram} - My DoB: {dob}"

        # TODO: get ckb balance
        balance = 0

        tgid = psql_db.update_member(telegram, ckb_address, balance, dob)

        try:
            tgbot.send_message(chat_id=tgid, text=f"🔔 Your telegram account has passed KYC")
            config.logger.info(f"Message sent to user ID: {telegram}")
        except Exception as e:
            config.logger.error(f"Failed to send message to user ID: {telegram}, Error: {e}")
    
        return jsonify(success=True)
    
if __name__ == '__main__':
    app.run(debug=True, host="0.0.0.0", port=8081)