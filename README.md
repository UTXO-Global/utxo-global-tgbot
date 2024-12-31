```bash
curl -X POST http://localhost:5000/chat -d '{"msg": "Hello, how are you?"}'
```

# Install ollama

```bash
curl -fsSL https://ollama.com/install.sh | sh
pip install huggingface_hub
huggingface-cli login
ollama run hf.co/bartowski/FuseChat-Llama-3.2-3B-Instruct-GGUF:Q6_K_L
```
