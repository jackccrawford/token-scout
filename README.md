# Token Scout

**Find free LLM models. Use them directly.**

An MCP server that knows which free-tier LLM APIs exist, which ones you have keys for, and which one fits your task. Returns endpoints — you call models directly. No proxy, no middleware, no latency tax.

1 tool. 5 minutes to first query. Works with Claude Code, Claude Desktop, or any MCP client.

---

## For AI Systems

Token-efficient reference:

| Tool | Args | Purpose |
|------|------|---------|
| `token_scout` | `query, prefer=""` | Find free models by name, size, provider, or capability |

### token_scout()

```
token_scout(query="reasoning")
→ DeepSeek R1 671B-MoE on OpenRouter, Grok 3 Mini on GitHub...

token_scout(query="fast classification")
→ Llama 3.1 8B on Groq, Llama 4 Scout on Cerebras...

token_scout(query="", prefer="context")
→ all models ranked by context window size

token_scout(query="")
→ status: which providers are configured, how many models available
```

**prefer** options: `quota` (most requests remaining), `speed` (fastest), `context` (largest window)

Returns: model ID, endpoint, API style, key env var, context window, strengths, quota. Everything you need to make the call yourself.

---

## Providers

All free tier. No credit card required unless noted.

| Provider | Models | Get a key |
|----------|--------|-----------|
| **Groq** | Llama 4 Scout/Maverick, Llama 3.3 70B, Llama 3.1 8B, Kimi K2, Qwen3 32B, GPT-OSS 120B | [console.groq.com](https://console.groq.com) |
| **Cerebras** | Llama 3.3 70B, Llama 4 Scout, Qwen3 32B | [cloud.cerebras.ai](https://cloud.cerebras.ai) |
| **Mistral** | Mistral Small 3.1 24B | [console.mistral.ai](https://console.mistral.ai) |
| **OpenRouter** | Gemma 3 27B, Llama 3.3 70B, Mistral Small 3.1, DeepSeek R1 | [openrouter.ai](https://openrouter.ai) |
| **GitHub Models** | GPT-4o, DeepSeek R1, Grok 3 Mini | [github.com/marketplace/models](https://github.com/marketplace/models) |
| **Google AI** | Gemini 2.0 Flash (1M context!) | [aistudio.google.com](https://aistudio.google.com) |

19 models across 6 providers. Sign up for the ones you want — Token Scout automatically detects which keys you have.

---

## Quick Start

```bash
# Clone and install
git clone https://github.com/jackccrawford/token-scout.git
cd token-scout
pip install -e .

# Add your API keys (use whichever providers you have)
export GROQ_API_KEY=gsk_...
export OPENROUTER_API_KEY=sk-or-...
export GITHUB_TOKEN=ghp_...
# etc.

# Test it
token-scout
```

### Add to Claude Code

```bash
claude mcp add token-scout -- token-scout
```

### Add to Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "token-scout": {
      "command": "token-scout"
    }
  }
}
```

Set API keys in your shell profile (`~/.zshrc`, `~/.bashrc`), or pass them in the config:

```json
{
  "mcpServers": {
    "token-scout": {
      "command": "token-scout",
      "env": {
        "GROQ_API_KEY": "gsk_...",
        "OPENROUTER_API_KEY": "sk-or-..."
      }
    }
  }
}
```

---

## How It Works

Token Scout is a registry, not a proxy. It knows:
- What free models exist across providers
- What API keys you have configured
- What each model is good at (strengths: fast, reasoning, code, classification, deep-context)
- Rate limits per provider

When you query, it searches the registry, filters to configured providers, ranks by relevance (and optionally by your `prefer` strategy), and returns everything you need to call the model directly.

**Zero inference overhead.** The model call goes straight from you to the provider. Token Scout just told you where to go.

---

## Why Not Just Bookmark the Docs?

You could. But your AI can't read your bookmarks. Token Scout gives any MCP-connected AI instant awareness of what's available for free. Instead of hardcoding model IDs, an agent can ask "what's good for reasoning?" and get a current answer.

It's also a single place to update when providers change their free tiers — update the registry, every connected AI benefits.

---

## Contributing

PRs welcome. Especially:
- New free-tier providers
- Updated rate limits or model additions
- Strength annotations (what models are actually good at)

---

## License

MIT
