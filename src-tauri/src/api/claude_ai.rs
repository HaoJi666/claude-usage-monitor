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

    let extra_usage = parse_extra_usage(data);

    // Format A: { five_hour: { utilization, resets_at }, seven_day: {...} }
    if let (Some(fh), Some(sd)) = (data.get("five_hour"), data.get("seven_day")) {
        if let (Some(five_hour), Some(seven_day)) = (parse_period(fh), parse_period(sd)) {
            return Some(ProUsageData {
                five_hour,
                seven_day,
                plan_type,
                extra_usage,
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
                    extra_usage,
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

pub fn parse_extra_usage(data: &serde_json::Value) -> Option<ExtraUsage> {
    // Try all plausible container key names (any may be the real one).
    const CONTAINERS: &[&str] = &[
        "extra_usage", "overage", "metered_usage", "pay_as_you_go",
        "addon_usage", "addons", "billing_usage", "credit_usage",
    ];
    // unwrap_or(data) so we also search the top-level object.
    let src = CONTAINERS.iter()
        .find_map(|k| data.get(*k))
        .unwrap_or(data);

    // Log every key inside the chosen source object so we can diagnose field-name mismatches.
    if let Some(obj) = src.as_object() {
        let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
        log::debug!("parse_extra_usage: src keys={:?}", keys);
    }

    // Spent amount (required — return None if absent).
    const SPENT_KEYS: &[&str] = &[
        "amount_spent", "spent", "total_spent", "amount", "current_spend",
        "amount_usd", "total_usd", "spend", "usage_amount", "charged_amount",
        "total_amount_spent", "metered_spend", "overage_amount",
    ];
    let spent = SPENT_KEYS.iter()
        .find_map(|k| src.get(*k).and_then(|v| v.as_f64()))?;

    const LIMIT_KEYS: &[&str] = &[
        "spend_limit", "limit", "monthly_limit", "spending_limit",
        "budget", "max_spend", "max_amount", "cap", "allowance",
        "total_limit", "period_limit",
    ];
    let limit = LIMIT_KEYS.iter()
        .find_map(|k| src.get(*k).and_then(|v| v.as_f64()))
        .unwrap_or(0.0);

    const BALANCE_KEYS: &[&str] = &[
        "balance", "current_balance", "remaining", "remaining_balance",
        "available_balance", "credit_balance", "credits", "available",
    ];
    let balance = BALANCE_KEYS.iter()
        .find_map(|k| src.get(*k).and_then(|v| v.as_f64()))
        .unwrap_or(0.0);

    let percent_used = if limit > 0.0 {
        (spent / limit * 100.0).min(100.0)
    } else {
        const PCT_KEYS: &[&str] = &["percent_used", "utilization", "percent", "usage_percent"];
        PCT_KEYS.iter()
            .find_map(|k| src.get(*k).and_then(|v| v.as_f64()))
            .unwrap_or(0.0)
    };

    const RESETS_KEYS: &[&str] = &[
        "resets_at", "reset_date", "reset_at", "period_end",
        "billing_period_end", "cycle_end", "next_reset",
    ];
    let resets_at = RESETS_KEYS.iter()
        .find_map(|k| src.get(*k).and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    const ENABLED_KEYS: &[&str] = &["enabled", "extra_usage_enabled", "active", "is_enabled", "on"];
    let enabled = ENABLED_KEYS.iter()
        .find_map(|k| src.get(*k).and_then(|v| v.as_bool()))
        .unwrap_or(false);

    const RELOAD_KEYS: &[&str] = &[
        "auto_reload", "auto_reload_enabled", "auto_refill", "automatic_reload",
    ];
    let auto_reload = RELOAD_KEYS.iter()
        .find_map(|k| src.get(*k).and_then(|v| v.as_bool()))
        .unwrap_or(false);

    Some(ExtraUsage {
        enabled,
        spent,
        limit,
        balance,
        percent_used,
        resets_at,
        auto_reload,
    })
}
