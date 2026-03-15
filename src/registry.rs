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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl Provider {
    pub fn has_key(&self) -> bool {
        // Local providers don't need keys
        if self.is_local() {
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

        Some(Model {
            id: id.to_string(),
            name: format!("{} ({})", id, name),
            params,
            // Ollama: unlimited local, use high sentinel values
            rpm: 999, rpd: 999_999, tpm: 999_999, tpd: 999_999_999,
            context_len: 0, // unknown until probed
            speed_tps: 0,
            strengths,
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

        Some(Model {
            id: id.to_string(),
            name: format!("{} (explora/llama.cpp)", id),
            params: String::new(), // llama.cpp doesn't report this
            rpm: 999, rpd: 999_999, tpm: 999_999, tpd: 999_999_999,
            context_len: 65_536, // default 4-slot mode
            speed_tps: 47,       // benchmarked: glm-4.7-flash Q4_K_M
            strengths: vec!["chat".into(), "deep-context".into(), "reasoning".into()],
        })
    }).collect()
}


pub fn build_registry() -> Vec<Provider> {
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
                },
                Model {
                    id: "meta-llama/llama-4-maverick-17b-128e-instruct".into(),
                    name: "Llama 4 Maverick 17B".into(),
                    params: "17B".into(),
                    rpm: 30, rpd: 1000, tpm: 6_000, tpd: 500_000,
                    context_len: 131_072, speed_tps: 0,
                    strengths: vec!["chat".into(), "reasoning".into()],
                },
                Model {
                    id: "llama-3.3-70b-versatile".into(),
                    name: "Llama 3.3 70B".into(),
                    params: "70B".into(),
                    rpm: 30, rpd: 1000, tpm: 12_000, tpd: 100_000,
                    context_len: 128_000, speed_tps: 0,
                    strengths: vec!["chat".into(), "code".into(), "reasoning".into()],
                },
                Model {
                    id: "llama-3.1-8b-instant".into(),
                    name: "Llama 3.1 8B".into(),
                    params: "8B".into(),
                    rpm: 30, rpd: 14_400, tpm: 6_000, tpd: 500_000,
                    context_len: 128_000, speed_tps: 0,
                    strengths: vec!["fast".into(), "classification".into(), "chat".into()],
                },
                Model {
                    id: "moonshotai/kimi-k2-instruct".into(),
                    name: "Kimi K2".into(),
                    params: "1T-MoE".into(),
                    rpm: 60, rpd: 1000, tpm: 10_000, tpd: 300_000,
                    context_len: 131_072, speed_tps: 0,
                    strengths: vec!["code".into(), "reasoning".into(), "chat".into()],
                },
                Model {
                    id: "qwen/qwen3-32b".into(),
                    name: "Qwen3 32B".into(),
                    params: "32B".into(),
                    rpm: 60, rpd: 1000, tpm: 6_000, tpd: 500_000,
                    context_len: 32_768, speed_tps: 0,
                    strengths: vec!["reasoning".into(), "code".into(), "chat".into()],
                },
                Model {
                    id: "openai/gpt-oss-120b".into(),
                    name: "GPT-OSS 120B".into(),
                    params: "120B".into(),
                    rpm: 30, rpd: 1000, tpm: 8_000, tpd: 200_000,
                    context_len: 128_000, speed_tps: 0,
                    strengths: vec!["chat".into(), "code".into(), "reasoning".into()],
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
                },
                Model {
                    id: "llama-4-scout-17b-16e-instruct".into(),
                    name: "Llama 4 Scout 17B".into(),
                    params: "17B".into(),
                    rpm: 30, rpd: 14_400, tpm: 60_000, tpd: 1_000_000,
                    context_len: 131_072, speed_tps: 0,
                    strengths: vec!["fast".into(), "chat".into(), "classification".into()],
                },
                Model {
                    id: "qwen-3-32b".into(),
                    name: "Qwen3 32B".into(),
                    params: "32B".into(),
                    rpm: 30, rpd: 14_400, tpm: 60_000, tpd: 1_000_000,
                    context_len: 32_768, speed_tps: 0,
                    strengths: vec!["reasoning".into(), "code".into(), "chat".into()],
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
                },
                Model {
                    id: "meta-llama/llama-3.3-70b-instruct:free".into(),
                    name: "Llama 3.3 70B".into(),
                    params: "70B".into(),
                    rpm: 20, rpd: 50, tpm: 40_000, tpd: 200_000,
                    context_len: 128_000, speed_tps: 0,
                    strengths: vec!["chat".into(), "code".into(), "reasoning".into()],
                },
                Model {
                    id: "mistralai/mistral-small-3.1-24b-instruct:free".into(),
                    name: "Mistral Small 3.1".into(),
                    params: "24B".into(),
                    rpm: 20, rpd: 50, tpm: 40_000, tpd: 200_000,
                    context_len: 32_000, speed_tps: 0,
                    strengths: vec!["fast".into(), "chat".into(), "code".into()],
                },
                Model {
                    id: "deepseek/deepseek-r1:free".into(),
                    name: "DeepSeek R1".into(),
                    params: "671B-MoE".into(),
                    rpm: 20, rpd: 50, tpm: 40_000, tpd: 200_000,
                    context_len: 64_000, speed_tps: 0,
                    strengths: vec!["reasoning".into(), "code".into(), "deep-context".into()],
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
                },
                Model {
                    id: "deepseek/DeepSeek-R1".into(),
                    name: "DeepSeek R1".into(),
                    params: "671B-MoE".into(),
                    rpm: 1, rpd: 8, tpm: 4_000, tpd: 32_000,
                    context_len: 64_000, speed_tps: 0,
                    strengths: vec!["reasoning".into(), "code".into()],
                },
                Model {
                    id: "xai/grok-3-mini".into(),
                    name: "Grok 3 Mini".into(),
                    params: "?".into(),
                    rpm: 2, rpd: 30, tpm: 4_000, tpd: 120_000,
                    context_len: 131_072, speed_tps: 0,
                    strengths: vec!["reasoning".into(), "chat".into()],
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
