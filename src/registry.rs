use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub name: String,
    pub endpoint: String,
    pub api_style: ApiStyle,
    pub models: Vec<Model>,
    pub api_key_env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApiStyle {
    OpenAI,
    Ollama,
    Google,
    Custom,
}

/// How a model exposes its reasoning/thinking process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReasoningFormat {
    /// Thinking is a separate API-level field, not in response text (Claude, Gemini)
    ApiSeparated,
    /// Thinking appears as inline tags in the response text (DeepSeek R1, Qwen3+)
    InlineTags,
    /// Model reasons internally but output is hidden (OpenAI o-series)
    Hidden,
    /// No reasoning/thinking capability
    None,
}

/// How a model handles tool/function calling
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolFormat {
    /// Anthropic tool_use blocks
    Anthropic,
    /// OpenAI function_calling / tools format
    OpenAIFunction,
    /// Ollama's tool wrapper (OpenAI-compatible, not all models support it)
    Ollama,
    /// No tool support
    None,
}

/// Whether tool support actually works reliably
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolReliability {
    /// Tested, works correctly
    Native,
    /// API claims support but untested or inconsistent
    Claimed,
    /// No tool support
    None,
}

/// Compatibility profile — what a caller needs to know to safely use a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Compatibility {
    pub reasoning_format: ReasoningFormat,
    /// The actual tag name if InlineTags (e.g. "think")
    #[serde(default)]
    pub reasoning_tag: String,
    pub tool_format: ToolFormat,
    pub tool_reliability: ToolReliability,
    /// Max output/completion tokens (0 = unknown or unlimited)
    #[serde(default)]
    pub max_completion: u32,
    /// Input modalities: "text", "text+image", "text+image+video"
    #[serde(default = "default_modality")]
    pub modality: String,
}

fn default_modality() -> String {
    "text".to_string()
}

impl Default for Compatibility {
    fn default() -> Self {
        Self {
            reasoning_format: ReasoningFormat::None,
            reasoning_tag: String::new(),
            tool_format: ToolFormat::None,
            tool_reliability: ToolReliability::None,
            max_completion: 0,
            modality: "text".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub params: String,
    pub rpm: u32,
    pub rpd: u32,
    pub tpm: u32,
    pub tpd: u32,
    /// Context window in tokens (0 = unknown)
    #[serde(default)]
    pub context_len: u32,
    /// Approximate eval speed in tok/s (0 = unknown)
    #[serde(default)]
    pub speed_tps: u32,
    /// What this model is good at: "fast", "reasoning", "code", "chat", "deep-context", "classification"
    #[serde(default)]
    pub strengths: Vec<String>,
    /// Cost per token for prompt/input (0.0 = free)
    #[serde(default)]
    pub prompt_cost: f64,
    /// Cost per token for completion/output (0.0 = free)
    #[serde(default)]
    pub completion_cost: f64,
    /// Compatibility profile for safe routing
    #[serde(default)]
    pub compat: Compatibility,
}

/// Infer compatibility profile from model ID and provider context.
/// Covers the 80% case from naming conventions. Override with explicit compat for edge cases.
pub fn infer_compatibility(model_id: &str, provider_name: &str, supported_params: &[String]) -> Compatibility {
    let id = model_id.to_lowercase();
    let prov = provider_name.to_lowercase();

    // --- Reasoning format ---
    let (reasoning_format, reasoning_tag) = if id.contains("deepseek") && (id.contains("r1") || id.contains("r2")) {
        (ReasoningFormat::InlineTags, "think".to_string())
    } else if id.contains("qwen3") || id.contains("qwen4") {
        // Qwen3+ family uses inline think tags when reasoning is enabled
        (ReasoningFormat::InlineTags, "think".to_string())
    } else if id.contains("o1") || id.contains("o3") || id.contains("o4") {
        // OpenAI o-series: internal reasoning, hidden from output
        (ReasoningFormat::Hidden, String::new())
    } else if prov.contains("anthropic") || id.contains("claude") {
        (ReasoningFormat::ApiSeparated, String::new())
    } else if id.contains("gemini") {
        (ReasoningFormat::ApiSeparated, String::new())
    } else if id.contains("think") || id.contains("reason") {
        // Generic reasoning models — assume inline tags as the common pattern
        (ReasoningFormat::InlineTags, "think".to_string())
    } else {
        (ReasoningFormat::None, String::new())
    };

    // --- Tool format ---
    let has_tools_param = supported_params.iter().any(|p| p == "tools" || p == "tool_choice");

    let (tool_format, tool_reliability) = if prov.contains("anthropic") || id.contains("claude") {
        (ToolFormat::Anthropic, ToolReliability::Native)
    } else if prov.starts_with("ollama") {
        if has_tools_param {
            (ToolFormat::Ollama, ToolReliability::Claimed)
        } else {
            (ToolFormat::None, ToolReliability::None)
        }
    } else if has_tools_param {
        // Most cloud providers use OpenAI-compatible function calling
        (ToolFormat::OpenAIFunction, ToolReliability::Claimed)
    } else {
        (ToolFormat::None, ToolReliability::None)
    };

    // --- Modality (default text, caller can override from API data) ---
    let modality = "text".to_string();

    Compatibility {
        reasoning_format,
        reasoning_tag,
        tool_format,
        tool_reliability,
        max_completion: 0, // set from API data when available
        modality,
    }
}

impl Provider {
    pub fn has_key(&self) -> bool {
        // Local providers don't need keys
        if self.is_local() {
            return true;
        }
        // Live-discovered providers are browsable without a key
        // (free models may not need auth; the key is checked at call time)
        if self.name.ends_with("-live") {
            return true;
        }
        env::var(&self.api_key_env).map(|k| !k.is_empty()).unwrap_or(false)
    }

    pub fn api_key(&self) -> Option<String> {
        if self.is_local() {
            return None;
        }
        env::var(&self.api_key_env).ok().filter(|k| !k.is_empty())
    }

    pub fn is_local(&self) -> bool {
        self.api_style == ApiStyle::Ollama || self.api_key_env.is_empty()
    }
}

/// Ollama host definition for constellation discovery
struct OllamaHost {
    name: &'static str,
    host_env: &'static str,
    port_env: Option<&'static str>,
    default_host: Option<&'static str>,
    default_port: u16,
}

const OLLAMA_HOSTS: &[OllamaHost] = &[
    OllamaHost { name: "localhost", host_env: "OLLAMA_HOST", port_env: Some("OLLAMA_PORT"), default_host: Some("127.0.0.1"), default_port: 11434 },
    OllamaHost { name: "mars", host_env: "MARS_HOST", port_env: Some("MARS_PORT"), default_host: None, default_port: 11434 },
    OllamaHost { name: "galaxy", host_env: "GALAXY_HOST", port_env: None, default_host: None, default_port: 11434 },
    OllamaHost { name: "lunar", host_env: "LUNAR_HOST", port_env: Some("LUNAR_PORT"), default_host: None, default_port: 11434 },
    OllamaHost { name: "scout", host_env: "SCOUT_HOST", port_env: Some("SCOUT_PORT"), default_host: None, default_port: 11434 },
];

/// Discover models from a single Ollama host via GET /api/tags
fn discover_ollama_host(name: &str, url: &str) -> Vec<Model> {
    let tags_url = format!("{}/api/tags", url);
    let resp = match ureq::get(&tags_url).timeout(std::time::Duration::from_secs(3)).call() {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let body: serde_json::Value = match resp.into_json() {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let models = match body.get("models").and_then(|m| m.as_array()) {
        Some(arr) => arr,
        None => return vec![],
    };

    models.iter().filter_map(|m| {
        let id = m.get("name")?.as_str()?;
        let size = m.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
        let details = m.get("details");
        let param_size = details
            .and_then(|d| d.get("parameter_size"))
            .and_then(|p| p.as_str())
            .unwrap_or("");
        let family = details
            .and_then(|d| d.get("family"))
            .and_then(|f| f.as_str())
            .unwrap_or("");

        let params = if !param_size.is_empty() {
            param_size.to_string()
        } else if size > 50_000_000_000 { "70B+".into() }
            else if size > 15_000_000_000 { "32B".into() }
            else if size > 5_000_000_000 { "8B".into() }
            else if size > 2_000_000_000 { "3B".into() }
            else { "?".into() };

        // Infer strengths from model name/family
        let mut strengths = vec!["chat".to_string()];
        let id_lower = id.to_lowercase();
        if id_lower.contains("code") || family == "codegemma" || family == "codellama" {
            strengths.push("code".to_string());
        }
        if id_lower.contains("r1") || id_lower.contains("think") || id_lower.contains("reason") {
            strengths.push("reasoning".to_string());
        }
        if id_lower.contains("embed") {
            strengths = vec!["embedding".to_string()];
        }

        let compat = infer_compatibility(id, name, &[]);

        Some(Model {
            id: id.to_string(),
            name: format!("{} ({})", id, name),
            params,
            // Ollama: unlimited local, use high sentinel values
            rpm: 999, rpd: 999_999, tpm: 999_999, tpd: 999_999_999,
            context_len: 0, // unknown until probed
            speed_tps: 0,
            strengths,
            prompt_cost: 0.0,
            completion_cost: 0.0,
            compat,
        })
    }).collect()
}

/// Discover all Ollama hosts in the constellation
pub fn discover_ollama_constellation() -> Vec<Provider> {
    let mut providers = Vec::new();

    for host_def in OLLAMA_HOSTS {
        let host_ip = match env::var(host_def.host_env).ok().filter(|h| !h.is_empty()) {
            Some(h) => h,
            None => match host_def.default_host {
                Some(d) => d.to_string(),
                None => continue,
            },
        };

        let port = host_def.port_env
            .and_then(|e| env::var(e).ok())
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(host_def.default_port);

        let url = format!("http://{}:{}", host_ip, port);
        let models = discover_ollama_host(host_def.name, &url);

        if !models.is_empty() {
            providers.push(Provider {
                name: format!("ollama-{}", host_def.name),
                endpoint: url,
                api_style: ApiStyle::Ollama,
                models,
                api_key_env: String::new(),
            });
        }
    }

    // Explora: nginx load-balanced Ollama on 11434, llama.cpp on 11440
    if let Ok(explora_host) = env::var("EXPLORA_HOST") {
        if !explora_host.is_empty() {
            // Single load-balanced Ollama endpoint (nginx least_conn across 4 containers)
            let ollama_url = format!("http://{}:11434", explora_host);
            let models = discover_ollama_host("explora", &ollama_url);
            if !models.is_empty() {
                providers.push(Provider {
                    name: "ollama-explora".into(),
                    endpoint: ollama_url,
                    api_style: ApiStyle::Ollama,
                    models,
                    api_key_env: String::new(),
                });
            }

            // llama.cpp server on port 11440 — OpenAI-compatible, multi-GPU
            let llamacpp_url = format!("http://{}:11440", explora_host);
            let llamacpp_models = discover_llamacpp(&llamacpp_url);
            if !llamacpp_models.is_empty() {
                providers.push(Provider {
                    name: "llamacpp-explora".into(),
                    endpoint: format!("{}/v1", llamacpp_url),
                    api_style: ApiStyle::OpenAI,
                    models: llamacpp_models,
                    api_key_env: String::new(),
                });
            }
        }
    }

    providers
}

/// Discover models from a llama.cpp server via GET /v1/models
fn discover_llamacpp(base_url: &str) -> Vec<Model> {
    let url = format!("{}/v1/models", base_url);
    let resp = match ureq::get(&url).timeout(std::time::Duration::from_secs(3)).call() {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let body: serde_json::Value = match resp.into_json() {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let models = match body.get("data").and_then(|d| d.as_array()) {
        Some(arr) => arr,
        None => return vec![],
    };

    models.iter().filter_map(|m| {
        let id = m.get("id")?.as_str()?;

        let compat = infer_compatibility(id, "explora", &[]);

        Some(Model {
            id: id.to_string(),
            name: format!("{} (explora/llama.cpp)", id),
            params: String::new(), // llama.cpp doesn't report this
            rpm: 999, rpd: 999_999, tpm: 999_999, tpd: 999_999_999,
            context_len: 65_536, // default 4-slot mode
            speed_tps: 47,       // benchmarked: glm-4.7-flash Q4_K_M
            strengths: vec!["chat".into(), "deep-context".into(), "reasoning".into()],
            prompt_cost: 0.0,
            completion_cost: 0.0,
            compat,
        })
    }).collect()
}

/// Discover free and cheap models from OpenRouter via GET /api/v1/models.
/// No auth needed for the listing. Filters to text-capable models only.
pub fn discover_openrouter_live() -> Vec<Model> {
    let url = "https://openrouter.ai/api/v1/models";
    let resp = match ureq::get(url).timeout(std::time::Duration::from_secs(10)).call() {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let body: serde_json::Value = match resp.into_json() {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let models = match body.get("data").and_then(|d| d.as_array()) {
        Some(arr) => arr,
        None => return vec![],
    };

    models.iter().filter_map(|m| {
        let id = m.get("id")?.as_str()?;
        let name = m.get("name")?.as_str()?;

        // Filter: must support text input
        let input_modalities = m.get("architecture")
            .and_then(|a| a.get("input_modalities"))
            .and_then(|im| im.as_array());
        let has_text = input_modalities
            .map(|mods| mods.iter().any(|v| v.as_str() == Some("text")))
            .unwrap_or(false);
        if !has_text {
            return None;
        }

        // Parse pricing
        let pricing = m.get("pricing")?;
        let prompt_cost: f64 = pricing.get("prompt")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let completion_cost: f64 = pricing.get("completion")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let context_len = m.get("context_length")
            .and_then(|c| c.as_u64())
            .unwrap_or(0) as u32;

        let max_completion = m.get("top_provider")
            .and_then(|tp| tp.get("max_completion_tokens"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0) as u32;

        // Supported parameters for tool inference
        let supported_params: Vec<String> = m.get("supported_parameters")
            .and_then(|sp| sp.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Modality string
        let modality = m.get("architecture")
            .and_then(|a| a.get("modality"))
            .and_then(|mo| mo.as_str())
            .unwrap_or("text->text");
        // Simplify: "text+image+video->text" → "text+image+video"
        let modality_input = modality.split("->").next().unwrap_or("text").to_string();

        // Infer compatibility from model ID
        let mut compat = infer_compatibility(id, "openrouter", &supported_params);
        compat.max_completion = max_completion;
        compat.modality = modality_input;

        // Infer strengths from description
        let desc = m.get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_lowercase();
        let mut strengths = vec!["chat".to_string()];
        if desc.contains("code") || desc.contains("coding") || desc.contains("programming") {
            strengths.push("code".to_string());
        }
        if desc.contains("reason") || desc.contains("thinking") {
            strengths.push("reasoning".to_string());
        }
        if context_len >= 100_000 {
            strengths.push("deep-context".to_string());
        }

        // Infer params from model name or description
        let params = infer_params_from_name(name);

        // OpenRouter rate limits for free models
        let (rpd, tpd) = if prompt_cost == 0.0 && completion_cost == 0.0 {
            (50, 200_000)  // conservative free-tier limits
        } else {
            (10_000, 10_000_000) // paid models have generous limits
        };

        Some(Model {
            id: id.to_string(),
            name: name.to_string(),
            params,
            rpm: 20, rpd, tpm: 40_000, tpd,
            context_len,
            speed_tps: 0,
            strengths,
            prompt_cost,
            completion_cost,
            compat,
        })
    }).collect()
}

/// Try to extract parameter count from model name (e.g. "Qwen3 32B" → "32B")
fn infer_params_from_name(name: &str) -> String {
    for word in name.split_whitespace() {
        let w = word.trim_end_matches(|c: char| !c.is_alphanumeric());
        let w_lower = w.to_lowercase();
        if w_lower.ends_with('b') || w_lower.ends_with('t') {
            let num_part = &w_lower[..w_lower.len()-1];
            if num_part.parse::<f64>().is_ok() {
                return w.to_string();
            }
        }
        // Handle MoE notation like "A4B" or "480B"
        if w_lower.contains('b') && w_lower.chars().any(|c| c.is_ascii_digit()) {
            return w.to_string();
        }
    }
    "?".to_string()
}

pub fn build_registry() -> Vec<Provider> {
    let mut providers = build_static_registry();
    // Post-process: infer compat and set zero cost for all static (free-tier) models
    for provider in &mut providers {
        for model in &mut provider.models {
            model.prompt_cost = 0.0;
            model.completion_cost = 0.0;
            model.compat = infer_compatibility(&model.id, &provider.name, &[]);
        }
    }
    providers
}

fn build_static_registry() -> Vec<Provider> {
    vec![
        Provider {
            name: "groq".into(),
            endpoint: "https://api.groq.com/openai/v1".into(),
            api_style: ApiStyle::OpenAI,
            api_key_env: "GROQ_API_KEY".into(),
            models: vec![
                Model {
                    id: "meta-llama/llama-4-scout-17b-16e-instruct".into(),
                    name: "Llama 4 Scout 17B".into(),
                    params: "17B".into(),
                    rpm: 30, rpd: 1000, tpm: 30_000, tpd: 500_000,
                    context_len: 131_072, speed_tps: 0,
                    strengths: vec!["fast".into(), "chat".into(), "classification".into()],
                ..Default::default()
                },
                Model {
                    id: "meta-llama/llama-4-maverick-17b-128e-instruct".into(),
                    name: "Llama 4 Maverick 17B".into(),
                    params: "17B".into(),
                    rpm: 30, rpd: 1000, tpm: 6_000, tpd: 500_000,
                    context_len: 131_072, speed_tps: 0,
                    strengths: vec!["chat".into(), "reasoning".into()],
                ..Default::default()
                },
                Model {
                    id: "llama-3.3-70b-versatile".into(),
                    name: "Llama 3.3 70B".into(),
                    params: "70B".into(),
                    rpm: 30, rpd: 1000, tpm: 12_000, tpd: 100_000,
                    context_len: 128_000, speed_tps: 0,
                    strengths: vec!["chat".into(), "code".into(), "reasoning".into()],
                ..Default::default()
                },
                Model {
                    id: "llama-3.1-8b-instant".into(),
                    name: "Llama 3.1 8B".into(),
                    params: "8B".into(),
                    rpm: 30, rpd: 14_400, tpm: 6_000, tpd: 500_000,
                    context_len: 128_000, speed_tps: 0,
                    strengths: vec!["fast".into(), "classification".into(), "chat".into()],
                ..Default::default()
                },
                Model {
                    id: "moonshotai/kimi-k2-instruct".into(),
                    name: "Kimi K2".into(),
                    params: "1T-MoE".into(),
                    rpm: 60, rpd: 1000, tpm: 10_000, tpd: 300_000,
                    context_len: 131_072, speed_tps: 0,
                    strengths: vec!["code".into(), "reasoning".into(), "chat".into()],
                ..Default::default()
                },
                Model {
                    id: "qwen/qwen3-32b".into(),
                    name: "Qwen3 32B".into(),
                    params: "32B".into(),
                    rpm: 60, rpd: 1000, tpm: 6_000, tpd: 500_000,
                    context_len: 32_768, speed_tps: 0,
                    strengths: vec!["reasoning".into(), "code".into(), "chat".into()],
                ..Default::default()
                },
                Model {
                    id: "openai/gpt-oss-120b".into(),
                    name: "GPT-OSS 120B".into(),
                    params: "120B".into(),
                    rpm: 30, rpd: 1000, tpm: 8_000, tpd: 200_000,
                    context_len: 128_000, speed_tps: 0,
                    strengths: vec!["chat".into(), "code".into(), "reasoning".into()],
                ..Default::default()
                },
            ],
        },
        Provider {
            name: "cerebras".into(),
            endpoint: "https://api.cerebras.ai/v1".into(),
            api_style: ApiStyle::OpenAI,
            api_key_env: "CEREBRAS_API_KEY".into(),
            models: vec![
                Model {
                    id: "llama-3.3-70b".into(),
                    name: "Llama 3.3 70B".into(),
                    params: "70B".into(),
                    rpm: 30, rpd: 14_400, tpm: 60_000, tpd: 1_000_000,
                    context_len: 128_000, speed_tps: 0,
                    strengths: vec!["fast".into(), "chat".into(), "code".into()],
                ..Default::default()
                },
                Model {
                    id: "llama-4-scout-17b-16e-instruct".into(),
                    name: "Llama 4 Scout 17B".into(),
                    params: "17B".into(),
                    rpm: 30, rpd: 14_400, tpm: 60_000, tpd: 1_000_000,
                    context_len: 131_072, speed_tps: 0,
                    strengths: vec!["fast".into(), "chat".into(), "classification".into()],
                ..Default::default()
                },
                Model {
                    id: "qwen-3-32b".into(),
                    name: "Qwen3 32B".into(),
                    params: "32B".into(),
                    rpm: 30, rpd: 14_400, tpm: 60_000, tpd: 1_000_000,
                    context_len: 32_768, speed_tps: 0,
                    strengths: vec!["reasoning".into(), "code".into(), "chat".into()],
                ..Default::default()
                },
            ],
        },
        Provider {
            name: "mistral".into(),
            endpoint: "https://api.mistral.ai/v1".into(),
            api_style: ApiStyle::OpenAI,
            api_key_env: "MISTRAL_API_KEY".into(),
            models: vec![
                Model {
                    id: "mistral-small-latest".into(),
                    name: "Mistral Small 3.1".into(),
                    params: "24B".into(),
                    rpm: 60, rpd: 14_400, tpm: 500_000, tpd: 10_000_000,
                    context_len: 32_000, speed_tps: 0,
                    strengths: vec!["fast".into(), "chat".into(), "code".into()],
                ..Default::default()
                },
            ],
        },
        Provider {
            name: "openrouter".into(),
            endpoint: "https://openrouter.ai/api/v1".into(),
            api_style: ApiStyle::OpenAI,
            api_key_env: "OPENROUTER_API_KEY".into(),
            models: vec![
                Model {
                    id: "google/gemma-3-27b-it:free".into(),
                    name: "Gemma 3 27B".into(),
                    params: "27B".into(),
                    rpm: 20, rpd: 50, tpm: 40_000, tpd: 200_000,
                    context_len: 8_192, speed_tps: 0,
                    strengths: vec!["chat".into(), "classification".into()],
                ..Default::default()
                },
                Model {
                    id: "meta-llama/llama-3.3-70b-instruct:free".into(),
                    name: "Llama 3.3 70B".into(),
                    params: "70B".into(),
                    rpm: 20, rpd: 50, tpm: 40_000, tpd: 200_000,
                    context_len: 128_000, speed_tps: 0,
                    strengths: vec!["chat".into(), "code".into(), "reasoning".into()],
                ..Default::default()
                },
                Model {
                    id: "mistralai/mistral-small-3.1-24b-instruct:free".into(),
                    name: "Mistral Small 3.1".into(),
                    params: "24B".into(),
                    rpm: 20, rpd: 50, tpm: 40_000, tpd: 200_000,
                    context_len: 32_000, speed_tps: 0,
                    strengths: vec!["fast".into(), "chat".into(), "code".into()],
                ..Default::default()
                },
                Model {
                    id: "deepseek/deepseek-r1:free".into(),
                    name: "DeepSeek R1".into(),
                    params: "671B-MoE".into(),
                    rpm: 20, rpd: 50, tpm: 40_000, tpd: 200_000,
                    context_len: 64_000, speed_tps: 0,
                    strengths: vec!["reasoning".into(), "code".into(), "deep-context".into()],
                ..Default::default()
                },
            ],
        },
        Provider {
            name: "github".into(),
            endpoint: "https://models.github.ai/inference".into(),
            api_style: ApiStyle::OpenAI,
            api_key_env: "GITHUB_TOKEN".into(),
            models: vec![
                Model {
                    id: "openai/gpt-4o".into(),
                    name: "GPT-4o".into(),
                    params: "?".into(),
                    rpm: 10, rpd: 50, tpm: 8_000, tpd: 400_000,
                    context_len: 128_000, speed_tps: 0,
                    strengths: vec!["chat".into(), "code".into(), "reasoning".into()],
                ..Default::default()
                },
                Model {
                    id: "deepseek/DeepSeek-R1".into(),
                    name: "DeepSeek R1".into(),
                    params: "671B-MoE".into(),
                    rpm: 1, rpd: 8, tpm: 4_000, tpd: 32_000,
                    context_len: 64_000, speed_tps: 0,
                    strengths: vec!["reasoning".into(), "code".into()],
                ..Default::default()
                },
                Model {
                    id: "xai/grok-3-mini".into(),
                    name: "Grok 3 Mini".into(),
                    params: "?".into(),
                    rpm: 2, rpd: 30, tpm: 4_000, tpd: 120_000,
                    context_len: 131_072, speed_tps: 0,
                    strengths: vec!["reasoning".into(), "chat".into()],
                ..Default::default()
                },
            ],
        },
        Provider {
            name: "google".into(),
            endpoint: "https://generativelanguage.googleapis.com/v1beta".into(),
            api_style: ApiStyle::Google,
            api_key_env: "GOOGLE_AI_API_KEY".into(),
            models: vec![
                Model {
                    id: "gemini-2.0-flash".into(),
                    name: "Gemini 2.0 Flash".into(),
                    params: "?".into(),
                    rpm: 15, rpd: 1500, tpm: 1_000_000, tpd: 10_000_000,
                    context_len: 1_048_576, speed_tps: 0,
                    strengths: vec!["fast".into(), "deep-context".into(), "chat".into(), "code".into()],
                ..Default::default()
                },
            ],
        },
    ]
}

/// Build a quick lookup: lowercase search terms → (provider_idx, model_idx)
pub fn build_index(registry: &[Provider]) -> HashMap<String, Vec<(usize, usize)>> {
    let mut index: HashMap<String, Vec<(usize, usize)>> = HashMap::new();

    for (pi, provider) in registry.iter().enumerate() {
        for (mi, model) in provider.models.iter().enumerate() {
            let loc = (pi, mi);
            // Index by: provider name, model id, model name, params, fragments
            let mut terms: Vec<String> = vec![
                provider.name.to_lowercase(),
                model.id.to_lowercase(),
                model.name.to_lowercase(),
                model.params.to_lowercase(),
            ];
            for s in &model.strengths {
                terms.push(s.to_lowercase());
            }
            for term in &terms {
                for word in term.split(|c: char| !c.is_alphanumeric()) {
                    if word.len() >= 2 {
                        index.entry(word.to_string()).or_default().push(loc);
                    }
                }
                index.entry(term.clone()).or_default().push(loc);
            }
        }
    }
    // Deduplicate
    for locs in index.values_mut() {
        locs.sort();
        locs.dedup();
    }
    index
}
