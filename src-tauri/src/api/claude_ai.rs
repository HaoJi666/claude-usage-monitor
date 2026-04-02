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
    // Try top-level or nested under "extra_usage" / "overage"
    let src = data
        .get("extra_usage")
        .or_else(|| data.get("overage"))
        .unwrap_or(data);

    // Must have at least a spend or balance field to be considered valid.
    let spent = src
        .get("amount_spent")
        .or_else(|| src.get("spent"))
        .or_else(|| src.get("total_spent"))
        .and_then(|v| v.as_f64())?;

    let limit = src
        .get("spend_limit")
        .or_else(|| src.get("limit"))
        .or_else(|| src.get("monthly_limit"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let balance = src
        .get("balance")
        .or_else(|| src.get("current_balance"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let percent_used = if limit > 0.0 {
        (spent / limit * 100.0).min(100.0)
    } else {
        src.get("percent_used")
            .or_else(|| src.get("utilization"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    };

    let resets_at = src
        .get("resets_at")
        .or_else(|| src.get("reset_date"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let enabled = src
        .get("enabled")
        .or_else(|| src.get("extra_usage_enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let auto_reload = src
        .get("auto_reload")
        .or_else(|| src.get("auto_reload_enabled"))
        .and_then(|v| v.as_bool())
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
