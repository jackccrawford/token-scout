"""
Token Scout — find free LLM models across providers.

Single MCP tool. Zero inference overhead — returns endpoints,
you call models directly. No proxy, no middleman.
"""

import os

from fastmcp import FastMCP

# =============================================================================
# Registry — free-tier models across providers
# =============================================================================

PROVIDERS = [
    {
        "name": "groq",
        "endpoint": "https://api.groq.com/openai/v1",
        "api_style": "openai",
        "key_env": "GROQ_API_KEY",
        "models": [
            {"id": "meta-llama/llama-4-scout-17b-16e-instruct", "name": "Llama 4 Scout 17B", "params": "17B", "context": 131072, "strengths": ["fast", "chat", "classification"], "rpm": 30, "rpd": 1000, "tpm": 30000},
            {"id": "meta-llama/llama-4-maverick-17b-128e-instruct", "name": "Llama 4 Maverick 17B", "params": "17B", "context": 131072, "strengths": ["chat", "reasoning"], "rpm": 30, "rpd": 1000, "tpm": 6000},
            {"id": "llama-3.3-70b-versatile", "name": "Llama 3.3 70B", "params": "70B", "context": 128000, "strengths": ["chat", "code", "reasoning"], "rpm": 30, "rpd": 1000, "tpm": 12000},
            {"id": "llama-3.1-8b-instant", "name": "Llama 3.1 8B", "params": "8B", "context": 128000, "strengths": ["fast", "classification", "chat"], "rpm": 30, "rpd": 14400, "tpm": 6000},
            {"id": "moonshotai/kimi-k2-instruct", "name": "Kimi K2", "params": "1T-MoE", "context": 131072, "strengths": ["code", "reasoning", "chat"], "rpm": 60, "rpd": 1000, "tpm": 10000},
            {"id": "qwen/qwen3-32b", "name": "Qwen3 32B", "params": "32B", "context": 32768, "strengths": ["reasoning", "code", "chat"], "rpm": 60, "rpd": 1000, "tpm": 6000},
            {"id": "openai/gpt-oss-120b", "name": "GPT-OSS 120B", "params": "120B", "context": 128000, "strengths": ["chat", "code", "reasoning"], "rpm": 30, "rpd": 1000, "tpm": 8000},
        ],
    },
    {
        "name": "cerebras",
        "endpoint": "https://api.cerebras.ai/v1",
        "api_style": "openai",
        "key_env": "CEREBRAS_API_KEY",
        "models": [
            {"id": "llama-3.3-70b", "name": "Llama 3.3 70B", "params": "70B", "context": 128000, "strengths": ["fast", "chat", "code"], "rpm": 30, "rpd": 14400, "tpm": 60000},
            {"id": "llama-4-scout-17b-16e-instruct", "name": "Llama 4 Scout 17B", "params": "17B", "context": 131072, "strengths": ["fast", "chat", "classification"], "rpm": 30, "rpd": 14400, "tpm": 60000},
            {"id": "qwen-3-32b", "name": "Qwen3 32B", "params": "32B", "context": 32768, "strengths": ["reasoning", "code", "chat"], "rpm": 30, "rpd": 14400, "tpm": 60000},
        ],
    },
    {
        "name": "mistral",
        "endpoint": "https://api.mistral.ai/v1",
        "api_style": "openai",
        "key_env": "MISTRAL_API_KEY",
        "models": [
            {"id": "mistral-small-latest", "name": "Mistral Small 3.1", "params": "24B", "context": 32000, "strengths": ["fast", "chat", "code"], "rpm": 60, "rpd": 14400, "tpm": 500000},
        ],
    },
    {
        "name": "openrouter",
        "endpoint": "https://openrouter.ai/api/v1",
        "api_style": "openai",
        "key_env": "OPENROUTER_API_KEY",
        "models": [
            {"id": "google/gemma-3-27b-it:free", "name": "Gemma 3 27B", "params": "27B", "context": 8192, "strengths": ["chat", "classification"], "rpm": 20, "rpd": 50, "tpm": 40000},
            {"id": "meta-llama/llama-3.3-70b-instruct:free", "name": "Llama 3.3 70B", "params": "70B", "context": 128000, "strengths": ["chat", "code", "reasoning"], "rpm": 20, "rpd": 50, "tpm": 40000},
            {"id": "mistralai/mistral-small-3.1-24b-instruct:free", "name": "Mistral Small 3.1", "params": "24B", "context": 32000, "strengths": ["fast", "chat", "code"], "rpm": 20, "rpd": 50, "tpm": 40000},
            {"id": "deepseek/deepseek-r1:free", "name": "DeepSeek R1", "params": "671B-MoE", "context": 64000, "strengths": ["reasoning", "code", "deep-context"], "rpm": 20, "rpd": 50, "tpm": 40000},
        ],
    },
    {
        "name": "github",
        "endpoint": "https://models.github.ai/inference",
        "api_style": "openai",
        "key_env": "GITHUB_TOKEN",
        "models": [
            {"id": "openai/gpt-4o", "name": "GPT-4o", "params": "?", "context": 128000, "strengths": ["chat", "code", "reasoning"], "rpm": 10, "rpd": 50, "tpm": 8000},
            {"id": "deepseek/DeepSeek-R1", "name": "DeepSeek R1", "params": "671B-MoE", "context": 64000, "strengths": ["reasoning", "code"], "rpm": 1, "rpd": 8, "tpm": 4000},
            {"id": "xai/grok-3-mini", "name": "Grok 3 Mini", "params": "?", "context": 131072, "strengths": ["reasoning", "chat"], "rpm": 2, "rpd": 30, "tpm": 4000},
        ],
    },
    {
        "name": "google",
        "endpoint": "https://generativelanguage.googleapis.com/v1beta",
        "api_style": "google",
        "key_env": "GOOGLE_AI_API_KEY",
        "models": [
            {"id": "gemini-2.0-flash", "name": "Gemini 2.0 Flash", "params": "?", "context": 1048576, "strengths": ["fast", "deep-context", "chat", "code"], "rpm": 15, "rpd": 1500, "tpm": 1000000},
        ],
    },
]

# =============================================================================
# Search index
# =============================================================================

def _build_index():
    """Build keyword → [(provider_idx, model_idx)] lookup."""
    index = {}
    for pi, prov in enumerate(PROVIDERS):
        for mi, model in enumerate(prov["models"]):
            loc = (pi, mi)
            terms = [
                prov["name"],
                model["id"].lower(),
                model["name"].lower(),
                model["params"].lower(),
            ] + [s.lower() for s in model.get("strengths", [])]
            for term in terms:
                for word in term.replace("/", " ").replace("-", " ").replace(":", " ").split():
                    if len(word) >= 2:
                        index.setdefault(word, []).append(loc)
                index.setdefault(term, []).append(loc)
    # Deduplicate
    for key in index:
        index[key] = list(set(index[key]))
    return index

INDEX = _build_index()

# =============================================================================
# Scout logic
# =============================================================================

def _has_key(provider):
    key = os.environ.get(provider["key_env"], "")
    return bool(key)

def _params_to_sortable(params: str) -> float:
    """Convert params string to a number for sorting. Smaller = faster."""
    p = params.lower().replace("-", "").replace("moe", "")
    try:
        if "t" in p:
            return float(p.replace("t", "")) * 1000
        if "b" in p:
            return float(p.replace("b", ""))
        return 999  # unknown → sort last for speed
    except ValueError:
        return 999

def _model_result(prov, model):
    """Build a result dict for a provider/model pair."""
    return {
        "provider": prov["name"],
        "model": model["id"],
        "model_name": model["name"],
        "params": model["params"],
        "endpoint": prov["endpoint"],
        "api_style": prov["api_style"],
        "api_key_env": prov["key_env"],
        "context_len": model.get("context", 0),
        "strengths": model.get("strengths", []),
        "quota": {
            "requests_per_day": model.get("rpd", 0),
            "tokens_per_minute": model.get("tpm", 0),
        },
    }

def _scout(query: str, prefer: str) -> dict:
    if not query and not prefer:
        # Status view — no query, no ranking preference
        providers_status = []
        total_configured = 0
        total_models = 0
        for p in PROVIDERS:
            configured = _has_key(p)
            if configured:
                total_configured += 1
                total_models += len(p["models"])
            providers_status.append({
                "name": p["name"],
                "configured": configured,
                "models": p["models"] if configured else [],
            })
        return {
            "providers": providers_status,
            "summary": f"{total_configured} providers configured, {total_models} models available",
        }

    if not query and prefer:
        # All configured models, ranked by prefer strategy
        results = []
        for prov in PROVIDERS:
            if not _has_key(prov):
                continue
            for model in prov["models"]:
                results.append(_model_result(prov, model))

        if prefer == "speed":
            results.sort(key=lambda r: (
                0 if "fast" in r["strengths"] else 1,
                _params_to_sortable(r["params"]),
            ))
        elif prefer == "context":
            results.sort(key=lambda r: -r.get("context_len", 0))
        elif prefer == "quota":
            results.sort(key=lambda r: -r["quota"]["requests_per_day"])

        return {
            "matches": results,
            "summary": f"{len(results)} models available across {len(set(r['provider'] for r in results))} providers",
        }

    # Search by query
    query_lower = query.lower()
    terms = query_lower.replace("/", " ").replace("-", " ").replace(":", " ").split()
    scores = {}
    for term in terms:
        for key, locs in INDEX.items():
            if term in key:
                for loc in locs:
                    scores[loc] = scores.get(loc, 0) + (2 if term == key else 1)

    # Filter to configured providers, sort by score
    results = []
    for (pi, mi), score in sorted(scores.items(), key=lambda x: -x[1]):
        prov = PROVIDERS[pi]
        if not _has_key(prov):
            continue
        model = prov["models"][mi]
        r = _model_result(prov, model)
        r["_score"] = score
        results.append(r)

    # Secondary sort by prefer (stable sort preserves relevance within ties)
    if prefer == "speed":
        results.sort(key=lambda r: (
            -r["_score"],
            0 if "fast" in r["strengths"] else 1,
            _params_to_sortable(r["params"]),
        ))
    elif prefer == "context":
        results.sort(key=lambda r: (-r["_score"], -r.get("context_len", 0)))
    elif prefer == "quota":
        results.sort(key=lambda r: (-r["_score"], -r["quota"]["requests_per_day"]))

    for r in results:
        r.pop("_score", None)

    return {
        "matches": results,
        "summary": f"{len(results)} models available across {len(set(r['provider'] for r in results))} providers",
    }

# =============================================================================
# MCP Server
# =============================================================================

mcp = FastMCP(
    name="Token Scout",
    instructions="Free LLM token gateway. Use token_scout() to find available free models "
                 "across providers (Groq, Cerebras, Mistral, OpenRouter, GitHub, Google).",
)

@mcp.tool(description="""
Find available free LLM models across providers.

Args:
    query: Search by model name, provider, size, capability, or strength.
           Examples: "llama", "deepseek", "70b", "groq", "reasoning", "fast", "code"
           Empty string returns status, or all models ranked if prefer is set.
    prefer: Optional ranking strategy.
           "quota"   — most requests per day first
           "speed"   — smallest/fastest models first (prefers "fast" strength)
           "context" — largest context window first

Returns:
    Matched models with endpoints, API style, context window, strengths, and quota.
    Use the returned endpoint + api_key_env to call the model directly.
""")
def token_scout(query: str = "", prefer: str = "") -> str:
    result = _scout(query, prefer)

    matches = result.get("matches", [])
    summary = result.get("summary", "")
    lines = []

    if not matches:
        providers = result.get("providers", [])
        if providers:
            for p in providers:
                status = "✓" if p.get("configured") else "✗"
                lines.append(f"  {status} {p['name']}: {len(p['models'])} models")
            lines.append(f"\n{summary}")
            return "\n".join(lines)
        return summary or "No models found"

    for m in matches:
        q = m.get("quota", {})
        ctx = m.get("context_len", 0)
        ctx_str = f"{ctx:,}" if ctx > 0 else "?"
        strengths = ", ".join(m.get("strengths", [])) or "general"

        lines.append(
            f"  {m['provider']}: {m['model_name']} ({m['params']})\n"
            f"    model: {m['model']}\n"
            f"    endpoint: {m['endpoint']}\n"
            f"    api_style: {m['api_style']}\n"
            f"    api_key_env: {m['api_key_env']}\n"
            f"    context: {ctx_str} | strengths: {strengths}\n"
            f"    quota: ~{q.get('requests_per_day', '?')} req/day, ~{q.get('tokens_per_minute', '?')} tok/min"
        )

    lines.append(f"\n{summary}")
    return "\n".join(lines)


def main():
    mcp.run()

if __name__ == "__main__":
    main()
