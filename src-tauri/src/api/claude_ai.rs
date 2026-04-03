use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodUsage {
    pub utilization: f64,
    pub resets_at: String,
    /// "five_hour" | "current_session" | "session"
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraUsage {
    pub enabled: bool,
    pub spent: f64,
    pub limit: f64,
    pub balance: f64,
    pub percent_used: f64,
    pub resets_at: String,
    pub auto_reload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProUsageData {
    pub five_hour: PeriodUsage,
    pub seven_day: PeriodUsage,
    /// MAX plan only: Sonnet-specific weekly limit ("Sonnet only" row)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seven_day_sonnet: Option<PeriodUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_usage: Option<ExtraUsage>,
    pub fetched_at: String,
}

/// Parse captured API response from claude.ai.
///
/// Supported response shapes (Pro and MAX):
///   A. { five_hour: {...}, seven_day: {...} }
///   B. { usage: { five_hour: {...}, seven_day: {...} } }
///   C. { current_session: {...}, seven_day: {...} }           ← MAX
///   D. { current_session: {...}, seven_day: {...},
///         seven_day_sonnet: {...} }                           ← MAX with Sonnet row
///   + nested variants of C/D inside a `usage` key
pub fn parse_usage(url: &str, data: &serde_json::Value) -> Option<ProUsageData> {
    log::debug!("parse_usage url={}", url);

    let plan_type = data
        .get("plan_type")
        .or_else(|| data.get("plan"))
        .or_else(|| data.get("subscription_plan"))
        .or_else(|| data.get("plan_tier"))
        .or_else(|| data.get("tier"))
        .and_then(|v| v.as_str())
        .map(String::from);

    // Field-name candidates for the short-term (session / 5-h) period.
    const SESSION_KEYS: &[&str] = &["five_hour", "current_session", "session"];
    // Field-name candidates for the weekly (all-models) period.
    const WEEKLY_KEYS: &[&str] = &["seven_day", "weekly", "all_models"];
    // Field-name candidates for the Sonnet-specific weekly period (MAX only).
    const SONNET_KEYS: &[&str] = &[
        "seven_day_sonnet",
        "sonnet_seven_day",
        "weekly_sonnet",
        "sonnet_only",
        "claude_sonnet_seven_day",
    ];

    // Try to parse from a given JSON object (either root or nested `usage`).
    let try_parse = |obj: &serde_json::Value| -> Option<ProUsageData> {
        let five_hour = SESSION_KEYS.iter().find_map(|k| {
            obj.get(*k).and_then(|v| {
                parse_period(v).map(|mut p| {
                    p.kind = k.to_string();
                    p
                })
            })
        })?;

        let seven_day = WEEKLY_KEYS.iter().find_map(|k| {
            obj.get(*k).and_then(|v| parse_period(v))
        })?;

        let seven_day_sonnet = SONNET_KEYS.iter().find_map(|k| {
            obj.get(*k).and_then(|v| parse_period(v))
        });

        Some(ProUsageData {
            five_hour,
            seven_day,
            seven_day_sonnet,
            plan_type: plan_type.clone(),
            extra_usage: None,
            fetched_at: Utc::now().to_rfc3339(),
        })
    };

    // Format A / C / D — flat root object
    if let Some(result) = try_parse(data) {
        return Some(result);
    }

    // Format B — nested under `usage` key
    if let Some(nested) = data.get("usage") {
        if let Some(result) = try_parse(nested) {
            return Some(result);
        }
    }

    log::warn!(
        "parse_usage: no matching format for url={} keys=[{}]",
        url,
        data.as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>().join(", "))
            .unwrap_or_default()
    );
    None
}

fn parse_period(v: &serde_json::Value) -> Option<PeriodUsage> {
    Some(PeriodUsage {
        utilization: v.get("utilization")?.as_f64()?,
        resets_at: v.get("resets_at")?.as_str()?.to_string(),
        kind: String::new(),
    })
}

/// Parse extra usage from the nested `extra_usage` object inside /usage endpoint.
pub fn parse_usage_extra(data: &serde_json::Value) -> Option<ExtraUsage> {
    let src = data.get("extra_usage")?;
    let used_credits = src.get("used_credits").and_then(|v| v.as_f64())?;
    let monthly_limit = src.get("monthly_limit").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let utilization = src.get("utilization").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let enabled = src.get("is_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    Some(ExtraUsage {
        enabled,
        spent: used_credits / 100.0,
        limit: monthly_limit / 100.0,
        balance: 0.0,
        percent_used: utilization,
        resets_at: String::new(),
        auto_reload: false,
    })
}

/// Parse prepaid credit balance and auto-reload flag from /prepaid/credits endpoint.
pub fn parse_prepaid_credits(data: &serde_json::Value) -> Option<(f64, bool)> {
    let amount = data.get("amount").and_then(|v| v.as_f64())?;
    let auto_reload = data
        .get("auto_reload_settings")
        .and_then(|s| s.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Some((amount / 100.0, auto_reload))
}

/// Try to extract a plan-type string from an account / subscription API response.
/// Returns None if the response doesn't look like account/subscription data.
pub fn parse_plan_type(data: &serde_json::Value) -> Option<String> {
    data.get("plan_type")
        .or_else(|| data.get("plan"))
        .or_else(|| data.get("subscription_plan"))
        .or_else(|| data.get("plan_tier"))
        .or_else(|| data.get("tier"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
}
