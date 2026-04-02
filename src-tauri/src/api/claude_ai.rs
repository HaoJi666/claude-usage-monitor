use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodUsage {
    pub utilization: f64,
    pub resets_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProUsageData {
    pub five_hour: PeriodUsage,
    pub seven_day: PeriodUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
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
