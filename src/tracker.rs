use chrono::{DateTime, Utc, Duration};
use serde::Serialize;
use std::collections::HashMap;

/// Tracks runtime quota consumption per (provider, model).
/// Resets daily. All in-memory — no persistence needed for overnight swarms.
#[derive(Debug)]
pub struct QuotaTracker {
    entries: HashMap<String, QuotaEntry>,
}

#[derive(Debug, Clone)]
struct QuotaEntry {
    requests_used: u32,
    tokens_used: u32,
    last_reset: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuotaStatus {
    pub requests_used: u32,
    pub requests_limit: u32,
    pub requests_remaining: u32,
    pub tokens_used: u32,
    pub tokens_limit: u32,
    pub tokens_remaining: u32,
    pub resets_at: String,
}

impl QuotaTracker {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn key(provider: &str, model_id: &str) -> String {
        format!("{}:{}", provider, model_id)
    }

    fn ensure_fresh(&mut self, key: &str) {
        let now = Utc::now();
        if let Some(entry) = self.entries.get(key) {
            // Reset if last reset was before midnight UTC today
            let today_midnight = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
            let today = DateTime::<Utc>::from_naive_utc_and_offset(today_midnight, Utc);
            if entry.last_reset < today {
                self.entries.insert(key.to_string(), QuotaEntry {
                    requests_used: 0,
                    tokens_used: 0,
                    last_reset: now,
                });
            }
        }
    }

    pub fn get_status(&mut self, provider: &str, model_id: &str, rpd: u32, tpd: u32) -> QuotaStatus {
        let key = Self::key(provider, model_id);
        self.ensure_fresh(&key);

        let entry = self.entries.entry(key).or_insert_with(|| QuotaEntry {
            requests_used: 0,
            tokens_used: 0,
            last_reset: Utc::now(),
        });

        let tomorrow = (Utc::now() + Duration::days(1))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        QuotaStatus {
            requests_used: entry.requests_used,
            requests_limit: rpd,
            requests_remaining: rpd.saturating_sub(entry.requests_used),
            tokens_used: entry.tokens_used,
            tokens_limit: tpd,
            tokens_remaining: tpd.saturating_sub(entry.tokens_used),
            resets_at: DateTime::<Utc>::from_naive_utc_and_offset(tomorrow, Utc)
                .to_rfc3339(),
        }
    }

    pub fn consume(&mut self, provider: &str, model_id: &str, requests: u32, tokens: u32) {
        let key = Self::key(provider, model_id);
        self.ensure_fresh(&key);

        let entry = self.entries.entry(key).or_insert_with(|| QuotaEntry {
            requests_used: 0,
            tokens_used: 0,
            last_reset: Utc::now(),
        });

        entry.requests_used += requests;
        entry.tokens_used += tokens;
    }

    pub fn reset_all(&mut self) {
        self.entries.clear();
    }

    pub fn has_quota(&mut self, provider: &str, model_id: &str, rpd: u32, tpd: u32) -> bool {
        let status = self.get_status(provider, model_id, rpd, tpd);
        status.requests_remaining > 0 && status.tokens_remaining > 0
    }
}
