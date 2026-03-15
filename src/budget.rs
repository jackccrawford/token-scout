use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BudgetRaw {
    pub session_pct: u32,
    pub session_resets_minutes: i64,
    pub weekly_all_pct: u32,
    pub weekly_all_resets: String,
    pub sonnet_pct: u32,
    pub sonnet_resets: String,
    pub extra_spent_usd: f64,
    pub extra_limit_usd: f64,
    pub extra_balance_usd: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BudgetState {
    pub scraped_at: String,
    pub raw: BudgetRaw,
    pub session_remaining_pct: u32,
    pub weekly_remaining_pct: u32,
    pub sonnet_remaining_pct: u32,
}

#[derive(Debug, Serialize)]
pub struct BudgetAdvice {
    pub recommendation: String,
    pub session_pace: String,
    pub weekly_pace: String,
    pub session_pct: u32,
    pub weekly_pct: u32,
    pub session_remaining_pct: u32,
    pub weekly_remaining_pct: u32,
    pub reason: String,
    pub stale: bool,
}

pub fn get_budget_advice() -> Option<BudgetAdvice> {
    let contents = std::fs::read_to_string("/tmp/claude-usage.json").ok()?;
    let state: BudgetState = serde_json::from_str(&contents).ok()?;

    let scraped_at = DateTime::parse_from_rfc3339(&state.scraped_at).ok()?;
    let scraped_at_utc: DateTime<Utc> = scraped_at.with_timezone(&Utc);
    let now = Utc::now();
    let age_minutes = (now - scraped_at_utc).num_minutes();
    let stale = age_minutes > 30;

    // Session pacing
    // Assume 5-hour (300 min) session window
    let minutes_remaining = state.raw.session_resets_minutes as f64;
    let ideal_pct = 90.0 * (1.0 - minutes_remaining / 300.0);
    let session_pct = state.raw.session_pct as f64;

    let session_pace = if session_pct < ideal_pct - 10.0 {
        "under"
    } else if session_pct > ideal_pct + 10.0 {
        "over"
    } else {
        "on_track"
    };

    // Weekly pacing
    // Parse reset day name to figure days elapsed in the week
    // weekly_all_resets is like "Sat 1:00 AM"
    let days_elapsed = days_elapsed_in_week(&state.raw.weekly_all_resets);
    let weekly_pct = state.raw.weekly_all_pct as f64;
    let weekly_target = 14.3 * days_elapsed as f64;

    let weekly_pace = if weekly_pct < 10.0 * days_elapsed as f64 {
        "under"
    } else if weekly_pct > 18.0 * days_elapsed as f64 {
        "over"
    } else {
        "on_track"
    };

    // Recommendation
    let recommendation = if state.raw.session_pct >= 90 {
        "free_only"
    } else if session_pace == "under" && weekly_pace == "under" {
        "burn_claude"
    } else if session_pace == "over" || weekly_pace == "over" {
        "conserve"
    } else if state.weekly_remaining_pct > 50 {
        "burn_claude"
    } else {
        "conserve"
    };

    // Reason string
    let reason = format!(
        "Session {}% (target {:.0}%), weekly {}% (target {:.0}% by day {}) — {}% weekly headroom, {}",
        state.raw.session_pct,
        ideal_pct,
        state.raw.weekly_all_pct,
        weekly_target,
        days_elapsed,
        state.weekly_remaining_pct,
        match recommendation {
            "burn_claude" => "burn it",
            "conserve" => "conserve",
            "free_only" => "session near limit, free only",
            _ => "unclear",
        }
    );

    Some(BudgetAdvice {
        recommendation: recommendation.to_string(),
        session_pace: session_pace.to_string(),
        weekly_pace: weekly_pace.to_string(),
        session_pct: state.raw.session_pct,
        weekly_pct: state.raw.weekly_all_pct,
        session_remaining_pct: state.session_remaining_pct,
        weekly_remaining_pct: state.weekly_remaining_pct,
        reason,
        stale,
    })
}

/// Parse the reset day string (e.g. "Sat 1:00 AM") to determine how many days
/// have elapsed since the weekly reset. Assumes the reset happens on that day
/// and that we count Mon=1 through Sun=7 as the week cycle.
fn days_elapsed_in_week(reset_day_str: &str) -> u32 {
    // Extract the day abbreviation (first token)
    let reset_day_abbr = reset_day_str.split_whitespace().next().unwrap_or("Sun");

    // Map day name to weekday number (Mon=0 .. Sun=6, matching chrono)
    let reset_weekday: u32 = match reset_day_abbr {
        "Mon" => 0,
        "Tue" => 1,
        "Wed" => 2,
        "Thu" => 3,
        "Fri" => 4,
        "Sat" => 5,
        "Sun" => 6,
        _ => 5, // default to Saturday if unrecognised
    };

    let today_weekday = Utc::now().weekday() as u32; // Mon=0 .. Sun=6 via chrono::Weekday
    // Days since the last reset: how far today is past reset_weekday in the week
    let elapsed = (today_weekday + 7 - reset_weekday) % 7;
    // Minimum 1 so we never divide by zero or claim day 0
    elapsed.max(1)
}
