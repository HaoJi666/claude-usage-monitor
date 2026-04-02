use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodUsage {
    pub utilization: f64,
    pub resets_at: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_usage: Option<ExtraUsage>,
    pub fetched_at: String,
}

/// Parse captured API response from claude.ai.
/// Handles multiple possible response shapes.
pub fn parse_usage(url: &str, data: &serde_json::Value) -> Option<ProUsageData> {
    log::debug!("Trying to parse usage from URL: {}", url);

    let plan_type = data
        .get("plan_type")
        .or_else(|| data.get("plan"))
        .or_else(|| data.get("subscription_plan"))
        .and_then(|v| v.as_str())
        .map(String::from);

    // Format A: { five_hour: { utilization, resets_at }, seven_day: {...} }
    if let (Some(fh), Some(sd)) = (data.get("five_hour"), data.get("seven_day")) {
        if let (Some(five_hour), Some(seven_day)) = (parse_period(fh), parse_period(sd)) {
            return Some(ProUsageData {
                five_hour,
                seven_day,
                plan_type,
                extra_usage: None, // populated separately in AppState::latest_extra
                fetched_at: Utc::now().to_rfc3339(),
            });
        }
    }

    // Format B: { usage: { five_hour: {...}, seven_day: {...} }, plan_type: ... }
    if let Some(usage) = data.get("usage") {
        if let (Some(fh), Some(sd)) = (usage.get("five_hour"), usage.get("seven_day")) {
            if let (Some(five_hour), Some(seven_day)) = (parse_period(fh), parse_period(sd)) {
                return Some(ProUsageData {
                    five_hour,
                    seven_day,
                    plan_type,
                    extra_usage: None,
                    fetched_at: Utc::now().to_rfc3339(),
                });
            }
        }
    }

    None
}

fn parse_period(v: &serde_json::Value) -> Option<PeriodUsage> {
    Some(PeriodUsage {
        utilization: v.get("utilization")?.as_f64()?,
        resets_at: v.get("resets_at")?.as_str()?.to_string(),
    })
}

/// Parse extra usage from the nested `extra_usage` object inside /usage endpoint.
/// Actual API fields: is_enabled, used_credits (cents), monthly_limit (cents), utilization (%)
pub fn parse_usage_extra(data: &serde_json::Value) -> Option<ExtraUsage> {
    let src = data.get("extra_usage")?;
    // used_credits is required — without it there's nothing meaningful to show.
    let used_credits = src.get("used_credits").and_then(|v| v.as_f64())?;
    let monthly_limit = src.get("monthly_limit").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let utilization = src.get("utilization").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let enabled = src.get("is_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    Some(ExtraUsage {
        enabled,
        spent: used_credits / 100.0,        // credits = cents → dollars
        limit: monthly_limit / 100.0,
        balance: 0.0,                        // patched later from /prepaid/credits
        percent_used: utilization,           // already in %
        resets_at: String::new(),           // patched later from /subscription_details
        auto_reload: false,                  // patched later from /prepaid/credits
    })
}

/// Parse prepaid credit balance and auto-reload flag from /prepaid/credits endpoint.
/// Returns (balance_dollars, auto_reload_enabled).
pub fn parse_prepaid_credits(data: &serde_json::Value) -> Option<(f64, bool)> {
    let amount = data.get("amount").and_then(|v| v.as_f64())?;
    let auto_reload = data
        .get("auto_reload_settings")
        .and_then(|s| s.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Some((amount / 100.0, auto_reload))
}
