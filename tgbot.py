from telegram import Update
from telegram.constants import ParseMode
from telegram.ext import Application, MessageHandler, filters, CallbackContext, CommandHandler
import os
from db import psql_db
import config

logger = config.logger

async def start(update: Update, context: CallbackContext) -> None:
    await update.message.reply_text("Hello! I'm CKB agent! Add me to a group and ask me questions!")

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
                await context.bot.send_message(
                    chat_id=tgid,
                    text=(
                        f"Hello {tgname}, welcome to the group! 👋\n"
                        "Please complete your KYC to get started.\n"
                        "Click the link below to begin:\n"
                        "[KYC Form](https://example.com/kyc)"
                    ),
                    # parse_mode=ParseMode.MARKDOWN
                )
                psql_db.insert_member(tgid, member.username)
                logger.info(f"Sent KYC message to {tgname} (ID: {tgid})")
            except Exception as e:
                logger.error(f"Could not message {tgname} (ID: {tgid}): {e}")

def main() -> None:
    # Create the Application and pass in the bot's token
    application = Application.builder().token(os.environ['TELEGRAM_TOKEN']).build()

    # Register handlers
    application.add_handler(CommandHandler("start", start))
    # application.add_handler(MessageHandler(filters.Mention(BOT_NAME), handle_mention))

    # Message handler for new members
    application.add_handler(MessageHandler(filters.StatusUpdate.NEW_CHAT_MEMBERS, new_member_handler))

    # Start the bot
    application.run_polling()

if __name__ == '__main__':
    main()
