use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::registry::Provider;
use crate::tracker::QuotaTracker;

#[derive(Debug, Serialize)]
pub struct ScoutResult {
    pub model: String,
    pub model_name: String,
    pub provider: String,
    pub endpoint: String,
    pub params: String,
    pub api_style: String,
    pub api_key_env: String,
    pub context_len: u32,
    pub speed_tps: u32,
    pub strengths: Vec<String>,
    pub quota: crate::tracker::QuotaStatus,
}

pub fn scout(
    query: &str,
    prefer: &str,
    registry: &[Provider],
    index: &HashMap<String, Vec<(usize, usize)>>,
    tracker: &mut QuotaTracker,
) -> Value {
    let query_lower = query.to_lowercase().trim().to_string();

    if query_lower.is_empty() {
        return status_all(registry, tracker);
    }

    // Score each (provider, model) by how many query words match
    let mut scores: HashMap<(usize, usize), u32> = HashMap::new();

    for word in query_lower.split_whitespace() {
        // Try exact word match first
        if let Some(locs) = index.get(word) {
            for loc in locs {
                *scores.entry(*loc).or_default() += 2;
            }
        }
        // Then substring match across all keys
        for (term, locs) in index.iter() {
            if term.contains(word) && term != word {
                for loc in locs {
                    *scores.entry(*loc).or_default() += 1;
                }
            }
        }
    }

    if scores.is_empty() {
        return json!({
            "matches": [],
            "summary": format!("No models matching '{}'", query)
        });
    }

    // First pass: collect candidates with relevance scores
    let mut ranked: Vec<((usize, usize), u32)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));

    let mut results: Vec<(ScoutResult, u32)> = Vec::new();

    for ((pi, mi), score) in ranked {
        let provider = &registry[pi];
        if !provider.has_key() {
            continue;
        }
        let model = &provider.models[mi];

        if !tracker.has_quota(&provider.name, &model.id, model.rpd, model.tpd) {
            continue;
        }

        let quota = tracker.get_status(&provider.name, &model.id, model.rpd, model.tpd);
        let api_style_str = match provider.api_style {
            crate::registry::ApiStyle::OpenAI => "openai",
            crate::registry::ApiStyle::Ollama => "ollama",
            crate::registry::ApiStyle::Google => "google",
            crate::registry::ApiStyle::Custom => "custom",
        };

        results.push((ScoutResult {
            model: model.id.clone(),
            model_name: model.name.clone(),
            provider: provider.name.clone(),
            endpoint: provider.endpoint.clone(),
            params: model.params.clone(),
            api_style: api_style_str.to_string(),
            api_key_env: provider.api_key_env.clone(),
            context_len: model.context_len,
            speed_tps: model.speed_tps,
            strengths: model.strengths.clone(),
            quota,
        }, score));
    }

    // Secondary sort by prefer strategy (stable sort preserves relevance order within ties)
    match prefer {
        "quota" => results.sort_by(|a, b| {
            b.1.cmp(&a.1) // primary: relevance
                .then(b.0.quota.requests_remaining.cmp(&a.0.quota.requests_remaining))
                .then(b.0.quota.tokens_remaining.cmp(&a.0.quota.tokens_remaining))
        }),
        "speed" => results.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then(b.0.speed_tps.cmp(&a.0.speed_tps))
        }),
        "context" => results.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then(b.0.context_len.cmp(&a.0.context_len))
        }),
        _ => {} // default: pure relevance (already sorted)
    }

    // Budget-aware filtering
    let budget_advice = if prefer == "budget" {
        crate::budget::get_budget_advice()
    } else {
        None
    };

    let is_free = |r: &ScoutResult| -> bool {
        r.api_key_env.is_empty()
            || r.provider.starts_with("ollama")
            || r.provider.starts_with("llamacpp")
    };

    let results: Vec<ScoutResult> = match budget_advice.as_ref().map(|a| a.recommendation.as_str()) {
        Some("free_only") => results
            .into_iter()
            .filter(|(r, _)| is_free(r))
            .map(|(r, _)| r)
            .collect(),
        Some("conserve") => {
            let mut free: Vec<ScoutResult> = Vec::new();
            let mut paid: Vec<ScoutResult> = Vec::new();
            for (r, _) in results {
                if is_free(&r) { free.push(r); } else { paid.push(r); }
            }
            free.extend(paid);
            free
        }
        _ => results.into_iter().map(|(r, _)| r).collect(),
    };

    let summary = format!("{} models available across {} providers",
        results.len(),
        results.iter().map(|r| &r.provider).collect::<std::collections::HashSet<_>>().len(),
    );

    let mut response = json!({
        "matches": results,
        "summary": summary
    });

    if let Some(advice) = budget_advice {
        response["budget"] = serde_json::to_value(advice).unwrap_or(Value::Null);
    }

    response
}

fn status_all(registry: &[Provider], tracker: &mut QuotaTracker) -> Value {
    let mut providers: Vec<Value> = Vec::new();

    for provider in registry {
        let has_key = provider.has_key();
        let mut models: Vec<Value> = Vec::new();

        for model in &provider.models {
            let quota = tracker.get_status(&provider.name, &model.id, model.rpd, model.tpd);
            models.push(json!({
                "id": model.id,
                "name": model.name,
                "params": model.params,
                "context_len": model.context_len,
                "speed_tps": model.speed_tps,
                "strengths": model.strengths,
                "quota": quota,
            }));
        }

        providers.push(json!({
            "name": provider.name,
            "endpoint": provider.endpoint,
            "configured": has_key,
            "models": models,
        }));
    }

    let configured = providers.iter().filter(|p| p["configured"].as_bool().unwrap_or(false)).count();
    let total_models: usize = registry.iter()
        .filter(|p| p.has_key())
        .map(|p| p.models.len())
        .sum();

    json!({
        "providers": providers,
        "summary": format!("{} providers configured, {} models available", configured, total_models)
    })
}
