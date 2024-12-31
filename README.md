```bash
curl -X POST http://localhost:5000/chat -d '{"msg": "Hello, how are you?"}'
```

# Zip file and move to server

```
zip -r utxo-global-tg-bot.zip . -x "./pyenv/*" -x "./.git/*"
scp utxo-global-tg-bot.zip ubuntu@ac:/home/ubuntu/utxo-global-tg-bot
```
