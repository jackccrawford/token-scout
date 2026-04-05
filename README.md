# Token Scout

**Find free and cheap LLM models. Route safely. Use them directly.**

A Rust CLI and MCP server that discovers available LLM models across cloud providers and local Ollama instances — in real time. Returns endpoints with compatibility profiles — you call models directly. No proxy, no middleware, no latency tax.

1 tool. 5 minutes to first query. Works with Claude Code, Claude Desktop, or any MCP client.

---

## For AI Systems

Token-efficient reference:

| Tool | Args | Purpose |
|------|------|---------|
| `token_scout` | `query, prefer="", require={}` | Find models by name, size, provider, capability, or compatibility |

### token_scout()

```
token_scout(query="reasoning code", require={"reasoning_format": "inline_tags", "min_context": 32000})
→ 33 models: Qwen3 Coder, DeepSeek R1 distills, Qwen3.6 Plus...

token_scout(query="fast classification")
→ Llama 3.1 8B on Groq, Llama 4 Scout on Cerebras...

token_scout(query="", prefer="context")
→ all models ranked by context window size

token_scout(query="")
→ status: providers, model counts, live discovery results
```

**prefer** options: `quota` (most requests remaining), `speed` (fastest), `context` (largest window), `budget` (Claude budget-aware)

**require** — hard filters applied before ranking:

| Field | Values | Purpose |
|-------|--------|---------|
| `reasoning_format` | `api_separated`, `inline_tags`, `hidden`, `none`, `any` | How the model exposes thinking |
| `tool_format` | `anthropic`, `openai_function`, `ollama`, `none`, `any` | Tool/function calling format |
| `tool_reliability` | `native`, `claimed`, `none`, `any` | Whether tool support actually works |
| `min_context` | integer (tokens) | Minimum context window |
| `min_completion` | integer (tokens) | Minimum output token limit |
| `modality` | `text`, `text+image`, etc. | Required input modality |

Returns: model ID, endpoint, API style, key env var, context window, strengths, pricing, compatibility profile, quota status. Everything you need to make the call yourself.

### Cost Gate

Set `TOKEN_SCOUT_MAX_COST` to control maximum cost per 1K tokens (prompt + completion averaged):

- `0` — free models only
- `0.001` — free + very cheap (default, ~$1/M tokens)
- `0.01` — includes mid-tier models
- Unset — defaults to `0.001`

---

## Providers

### Cloud (free tier, no credit card required unless noted)

| Provider | Models | Get a key |
|----------|--------|-----------|
| **Groq** | Llama 4 Scout/Maverick, Llama 3.3 70B, Kimi K2, Qwen3 32B, GPT-OSS 120B | [console.groq.com](https://console.groq.com) |
| **Cerebras** | Llama 3.3 70B, Llama 4 Scout, Qwen3 32B | [cloud.cerebras.ai](https://cloud.cerebras.ai) |
| **Mistral** | Mistral Small 3.1 24B | [console.mistral.ai](https://console.mistral.ai) |
| **OpenRouter** | 28+ free models, 600+ paid — **live discovery** | [openrouter.ai](https://openrouter.ai) |
| **GitHub Models** | GPT-4o, DeepSeek R1, Grok 3 Mini | [github.com/marketplace/models](https://github.com/marketplace/models) |
| **Google AI** | Gemini 2.0 Flash (1M context) | [aistudio.google.com](https://aistudio.google.com) |

### Local (Ollama constellation — auto-discovered)

Token Scout probes your local network for Ollama instances. Set env vars to point to your machines:

| Env var | Default | Purpose |
|---------|---------|---------|
| `OLLAMA_HOST` | `127.0.0.1` | Local Ollama |
| `MARS_HOST` | — | Fleet home |
| `GALAXY_HOST` | — | GPU inference |
| `LUNAR_HOST` | — | Light inference |
| `EXPLORA_HOST` | — | Heavy compute (4x GPU, nginx load-balanced) |

Local models are free (electricity only) and have unlimited quota.

### Live Discovery

OpenRouter models are discovered in real time via `GET /api/v1/models`. No API key needed for discovery. Free models change frequently — Token Scout catches them as they appear.

---

## Quick Start

### Rust CLI (recommended)

```bash
git clone https://github.com/jackccrawford/token-scout.git
cd token-scout
cargo build --release

# JSON-RPC over stdin/stdout
echo '{"jsonrpc":"2.0","id":1,"method":"scout","params":{"query":"reasoning"}}' | ./target/release/token-scout
```

### Python MCP Server

```bash
pip install -e .

# Add to Claude Code
claude mcp add token-scout -- token-scout

# Test it
token-scout
```

### Add to Claude Desktop

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

Set API keys in your shell profile (`~/.zshrc`, `~/.bashrc`), or pass them in the config.

---

## How It Works

Token Scout combines three discovery layers:

1. **Static registry** — curated free-tier models across 6 cloud providers (fallback)
2. **Ollama constellation** — auto-discovers models on local network machines
3. **OpenRouter live** — real-time discovery of 600+ models with pricing

When you query, Token Scout:
- Discovers available models (lazy, on first call)
- Filters by cost gate (`TOKEN_SCOUT_MAX_COST`)
- Filters by compatibility requirements (`require`)
- Filters by quota availability
- Ranks by relevance and `prefer` strategy
- Returns everything you need to call the model directly

### Compatibility Profiles

Every model gets a compatibility profile — inferred from model family and provider metadata:

| Field | What it tells you |
|-------|-------------------|
| `reasoning_format` | How thinking is exposed: `api_separated` (Claude, Gemini), `inline_tags` (DeepSeek R1, Qwen3+), `hidden` (OpenAI o-series), `none` |
| `reasoning_tag` | The actual tag name if inline (e.g. `think`) |
| `tool_format` | `anthropic`, `openai_function`, `ollama`, `none` |
| `tool_reliability` | `native` (tested), `claimed` (API says yes), `none` |
| `max_completion` | Output token limit |
| `modality` | Input modalities: `text`, `text+image`, etc. |

This prevents routing to incompatible models — no accidental context clipping, no thinking tag corruption, no broken tool calls.

### Budget Awareness

Token Scout reads `/tmp/claude-usage.json` (from `scripts/scrape-claude-usage.sh`) to track Claude session and weekly usage. When budget is tight, scout prioritizes free and local models automatically.

---

## Why Not Just Bookmark the Docs?

You could. But:
- Your AI can't read your bookmarks
- Free models on OpenRouter change hourly
- Compatibility matters — not every model handles tools or reasoning the same way
- Local Ollama models are free but invisible without discovery

Token Scout gives any MCP-connected AI instant awareness of what's available, what's compatible, and what it costs. The registry updates itself.

---

## Contributing

PRs welcome. Especially:
- New free-tier providers
- Compatibility profile corrections (tested tool support, reasoning format verification)
- Ollama host configurations for different network setups
- Budget integration improvements

---

## License

MIT
