# Token Scout

**Live LLM model discovery for AI agents. Free and cheap inference, routed safely.**

Token Scout discovers LLM models in real time — querying cloud providers and probing local Ollama instances every time you ask. It returns endpoints with compatibility profiles and live pricing so your agent can route tasks to the cheapest viable model without breaking tool calls, clipping context, or corrupting reasoning formats.

Built for autonomous AI agents, agentic frameworks, and multi-model workflows. Works with [Claude Code](https://claude.ai/claude-code), [OpenClaw](https://github.com/openclaw), [Hermes](https://github.com/nousresearch), [LangChain](https://github.com/langchain-ai/langchain), [CrewAI](https://github.com/crewai), or any system that needs to pick a model at runtime.

**No proxy. No middleware. No latency tax.** Token Scout tells your agent where to go. The agent calls the model directly.

---

## Why Your Agent Needs This

AI agents that hardcode model IDs are leaving money on the table. Right now there are **28+ free models on OpenRouter alone** — including Qwen3 Coder 480B, Nemotron 120B, and DeepSeek R1. Tomorrow that number will be different.

Token Scout gives your agent:
- **Live model discovery** — queries OpenRouter, Groq, Cerebras, Mistral, GitHub, Google, and local Ollama instances in real time
- **Compatibility filtering** — won't route to a model that breaks your tool calls, clips your context, or uses incompatible reasoning tags
- **Cost control** — set a max cost per 1K tokens; free models only, cheap models, or no limit
- **Quota tracking** — tracks requests and tokens consumed per provider, filters out exhausted models

One API call. Your agent gets back a ranked list of models that are available right now, compatible with the task, and within budget.

---

## For AI Systems

Token-efficient reference:

| Tool | Args | Purpose |
|------|------|---------|
| `token_scout` | `query, prefer="", require={}` | Find models by name, size, provider, capability, or compatibility |

### token_scout()

```
token_scout(query="reasoning code", require={"reasoning_format": "inline_tags", "min_context": 32000})
-> 33 models: Qwen3 Coder, DeepSeek R1 distills, Qwen3.6 Plus...

token_scout(query="fast classification")
-> Llama 3.1 8B on Groq, Llama 4 Scout on Cerebras...

token_scout(query="", prefer="context")
-> all models ranked by context window size

token_scout(query="")
-> status: providers, model counts, live discovery results
```

**prefer** options: `quota` (most requests remaining), `speed` (fastest), `context` (largest window), `budget` (Claude budget-aware)

**require** — hard constraints applied before ranking:

| Field | Values | Purpose |
|-------|--------|---------|
| `reasoning_format` | `api_separated`, `inline_tags`, `hidden`, `none`, `any` | How the model exposes chain-of-thought |
| `tool_format` | `anthropic`, `openai_function`, `ollama`, `none`, `any` | Tool/function calling format |
| `tool_reliability` | `native`, `claimed`, `none`, `any` | Whether tool support actually works |
| `min_context` | integer (tokens) | Minimum context window |
| `min_completion` | integer (tokens) | Minimum output token limit |
| `modality` | `text`, `text+image`, etc. | Required input modality |

Returns: model ID, endpoint, API style, key env var, context window, strengths, pricing, compatibility profile, quota status. Everything your agent needs to make the call.

### Cost Gate

Set `TOKEN_SCOUT_MAX_COST` to control maximum cost per 1K tokens (prompt + completion averaged):

- `0` — free models only
- `0.001` — free + very cheap (default, ~$1/M tokens)
- `0.01` — includes mid-tier models
- Unset — defaults to `0.001`

---

## The Problem Token Scout Solves

Agents that route tasks to LLMs face three compatibility walls:

1. **Tool format fragmentation** — Anthropic, OpenAI, and Ollama all handle function calling differently. Routing to the wrong format breaks your agent's tool chain.
2. **Context window clipping** — sending 200K tokens to a model with 32K context doesn't degrade gracefully. It's catastrophic data loss.
3. **Reasoning tag corruption** — Claude uses API-separated thinking. DeepSeek R1 and Qwen3 use inline `<think>` tags. Mixing these mid-workflow corrupts the session.

Token Scout profiles every model for these compatibility dimensions and filters before ranking. Your agent can't accidentally route to a model that will break it.

---

## Providers

### Cloud (free tier, no credit card required unless noted)

| Provider | Models | Get a key |
|----------|--------|-----------|
| **Groq** | Llama 4 Scout/Maverick, Llama 3.3 70B, Kimi K2, Qwen3 32B, GPT-OSS 120B | [console.groq.com](https://console.groq.com) |
| **Cerebras** | Llama 3.3 70B, Llama 4 Scout, Qwen3 32B | [cloud.cerebras.ai](https://cloud.cerebras.ai) |
| **Mistral** | Mistral Small 3.1 24B | [console.mistral.ai](https://console.mistral.ai) |
| **OpenRouter** | 28+ free, 600+ paid — **live discovery** | [openrouter.ai](https://openrouter.ai) |
| **GitHub Models** | GPT-4o, DeepSeek R1, Grok 3 Mini | [github.com/marketplace/models](https://github.com/marketplace/models) |
| **Google AI** | Gemini 2.0 Flash (1M context) | [aistudio.google.com](https://aistudio.google.com) |

### Local (Ollama constellation — auto-discovered)

Token Scout probes your local network for running Ollama instances. Set env vars to point to your machines:

| Env var | Default | Purpose |
|---------|---------|---------|
| `OLLAMA_HOST` | `127.0.0.1` | Local Ollama |
| `MARS_HOST` | — | Additional host |
| `GALAXY_HOST` | — | GPU inference |
| `LUNAR_HOST` | — | Light inference |
| `EXPLORA_HOST` | — | Heavy compute (multi-GPU, nginx load-balanced) |

Local models are free (electricity only) and have unlimited quota.

### Live Discovery

OpenRouter models are discovered in real time via `GET /api/v1/models`. No API key needed for discovery — free models are browsable immediately. Models and pricing change frequently; Token Scout catches them as they appear and disappear.

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

Token Scout discovers models live. Every query reflects what's actually available right now — not what was available when the code was last updated.

Three discovery layers run on first query:

1. **OpenRouter live** — queries the OpenRouter API for all available models with real-time pricing. Free models appear and disappear hourly; Token Scout catches them as they come and go.
2. **Ollama constellation** — probes your local network for running Ollama instances and inventories their loaded models.
3. **Static fallback** — a curated set of known free-tier providers (Groq, Cerebras, Mistral, GitHub, Google) for when live discovery is unavailable.

After discovery, every query:
- Filters by cost gate (`TOKEN_SCOUT_MAX_COST`)
- Filters by compatibility requirements (`require`)
- Filters by quota availability
- Ranks by relevance and `prefer` strategy
- Returns everything your agent needs to call the model directly

### Compatibility Profiles

Every discovered model gets a compatibility profile — inferred from model family, provider metadata, and live API fields:

| Field | What it tells your agent |
|-------|--------------------------|
| `reasoning_format` | How thinking is exposed: `api_separated` (Claude, Gemini), `inline_tags` (DeepSeek R1, Qwen3+), `hidden` (OpenAI o-series), `none` |
| `reasoning_tag` | The actual tag name if inline (e.g. `think`) — so your agent can parse or strip it |
| `tool_format` | `anthropic`, `openai_function`, `ollama`, `none` |
| `tool_reliability` | `native` (tested), `claimed` (API says yes), `none` |
| `max_completion` | Output token limit |
| `modality` | Input modalities: `text`, `text+image`, etc. |

### Budget Awareness

Token Scout reads `/tmp/claude-usage.json` (from `scripts/scrape-claude-usage.sh`) to track Claude session and weekly usage. When budget is tight, scout prioritizes free and local models automatically.

---

## Use Cases

- **Agentic coding assistants** — route sub-tasks (summarize, search, draft) to free models while the main agent stays on a premium model
- **Multi-model pipelines** — pick the right model for each stage: fast/cheap for classification, reasoning-capable for analysis, deep-context for synthesis
- **Cost optimization** — stop paying for inference on tasks that free models handle fine
- **Local-first AI** — discover and use Ollama models on your own hardware before touching cloud APIs
- **Fleet coordination** — multiple agents share a Token Scout instance, quota tracking prevents any single agent from exhausting a provider

---

## Contributing

PRs welcome. Especially:
- New provider integrations (live discovery endpoints)
- Compatibility profile corrections (tested tool support, reasoning format verification)
- Ollama host configurations for different network setups
- Budget integration improvements

---

## License

MIT
