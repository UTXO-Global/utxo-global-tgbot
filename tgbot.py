from telegram import Update
from telegram.constants import ParseMode
from telegram.ext import Application, MessageHandler, filters, CallbackContext, CommandHandler
import os
from bot import ask_bot
from db import psql_db
import config

logger = config.logger

BOT_NAME = "ckb-tgbot"

async def start(update: Update, context: CallbackContext) -> None:
    await update.message.reply_text("Hello! I'm CKB agent!\nI will help you do KYC on telegram. Please ask me anything.")

# Configuration
async def new_member_handler(update: Update, context: CallbackContext):
    logger.info(f"New member joined")
    print(update)
    """Handles new members in the group."""
    if update.message.new_chat_members:
        for member in update.message.new_chat_members:
            tgid = member.id
            tgname = member.username or member.full_name
            try:
                # Send a private message to the user
                kyc_url = os.environ['KYC_LINK']
                await context.bot.send_message(
                    chat_id=tgid,
                    text=(
                        f"Hello {tgname}, welcome to the group! 👋\n"
                        "Please complete your KYC to get started.\n"
                        "Click the link below to begin:\n"
                        f"[KYC Form]({kyc_url})"
                        "Please ask me anything if you need any help\n"
                    ),
                    # parse_mode=ParseMode.MARKDOWN
                )
                psql_db.insert_member(tgid, member.username)
                logger.info(f"Sent KYC message to {tgname} (ID: {tgid})")
            except Exception as e:
                logger.error(f"Could not message {tgname} (ID: {tgid}): {e}")

async def handle_direct_message(update: Update, context: CallbackContext):
    """Handle direct messages from users."""
    if update.message.chat.type == "private":
        user = update.message.from_user
        message = update.message.text
        # Send a response when receiving a direct message
        reply = ask_bot(message, BOT_NAME, user)
        await update.message.reply_text(reply)

def main() -> None:
    # Create the Application and pass in the bot's token
    application = Application.builder().token(os.environ['TELEGRAM_TOKEN']).build()

    # Register handlers
    application.add_handler(CommandHandler("start", start))

    # Message handler for new members
    application.add_handler(MessageHandler(filters.StatusUpdate.NEW_CHAT_MEMBERS, new_member_handler))
    application.add_handler(MessageHandler(filters.TEXT & ~filters.COMMAND, handle_direct_message))

    # Start the bot
    application.run_polling()

if __name__ == '__main__':
    main()
