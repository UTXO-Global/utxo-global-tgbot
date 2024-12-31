import requests
from telegram import Update
from telegram.ext import CommandHandler, MessageHandler, filters, CallbackContext, Application
from bot import ask_bot
import config
import os

# Configuration
BOT_NAME = "cotiagent_bot"

async def start(update: Update, context: CallbackContext) -> None:
    await update.message.reply_text("Hello! I'm Coti Agent! Add me to a group and ask me questions!")

async def handle_mention(update: Update, context: CallbackContext) -> None:
    user_msg = update.message.text
    user_address = update.message.from_user.full_name
    reply = ask_bot(user_msg, BOT_NAME, user_address)
    await update.message.reply_text(reply)

def main() -> None:
    # Create the Application and pass in the bot's token
    application = Application.builder().token(os.environ['TELEGRAM_TOKEN']).build()

    # Register handlers
    application.add_handler(CommandHandler("start", start))
    application.add_handler(MessageHandler(filters.Mention(BOT_NAME), handle_mention))

    # Start the bot
    application.run_polling()

if __name__ == '__main__':
    main()
