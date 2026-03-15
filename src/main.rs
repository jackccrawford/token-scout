mod budget;
mod registry;
mod scout;
mod tracker;

use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use registry::{build_index, build_registry, discover_ollama_constellation};
use tracker::QuotaTracker;

fn main() {
    let mut registry = build_registry();
    let mut index = build_index(&registry);
    let mut tracker = QuotaTracker::new();
    let mut discovered = false;

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err = json!({
                    "jsonrpc": "2.0",
                    "error": {"code": -32700, "message": format!("Parse error: {}", e)},
                    "id": null
                });
                let _ = writeln!(stdout, "{}", err);
                let _ = stdout.flush();
                continue;
            }
        };

        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(json!({}));

        // Lazy constellation discovery — runs once on first scout/status call
        if !discovered && (method == "scout" || method == "status") {
            let ollama_providers = discover_ollama_constellation();
            registry.extend(ollama_providers);
            index = build_index(&registry);
            discovered = true;
        }

        let result = match method {
            "scout" => {
                let query = params.get("query").and_then(|q| q.as_str()).unwrap_or("");
                let prefer = params.get("prefer").and_then(|p| p.as_str()).unwrap_or("");
                Ok(scout::scout(query, prefer, &registry, &index, &mut tracker))
            }
            "status" => {
                Ok(scout::scout("", "", &registry, &index, &mut tracker))
            }
            "consume" => {
                let provider = params.get("provider").and_then(|v| v.as_str()).unwrap_or("");
                let model_id = params.get("model").and_then(|v| v.as_str()).unwrap_or("");
                let requests = params.get("requests").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                let tokens = params.get("tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

                if provider.is_empty() || model_id.is_empty() {
                    Err("missing required: provider, model".to_string())
                } else {
                    tracker.consume(provider, model_id, requests, tokens);
                    Ok(json!({"consumed": true, "requests": requests, "tokens": tokens}))
                }
            }
            "reset" => {
                tracker.reset_all();
                Ok(json!({"reset": true}))
            }
            "budget" => {
                match budget::get_budget_advice() {
                    Some(advice) => Ok(serde_json::to_value(advice).unwrap_or(json!({"error": "serialize failed"}))),
                    None => Err("No budget data — run scrape-claude-usage.sh first".to_string()),
                }
            }
            "discover" => {
                let ollama_providers = discover_ollama_constellation();
                let count = ollama_providers.len();
                registry.extend(ollama_providers);
                index = build_index(&registry);
                discovered = true;
                Ok(json!({"discovered": true, "new_providers": count, "total_providers": registry.len()}))
            }
            _ => Err(format!("Unknown method: {}", method)),
        };

        let response = match result {
            Ok(value) => json!({
                "jsonrpc": "2.0",
                "result": value,
                "id": id
            }),
            Err(msg) => json!({
                "jsonrpc": "2.0",
                "error": {"code": -32000, "message": msg},
                "id": id
            }),
        };

        let _ = writeln!(stdout, "{}", response);
        let _ = stdout.flush();
    }
}
