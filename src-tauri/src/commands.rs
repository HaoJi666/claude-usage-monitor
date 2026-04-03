use tauri::{Emitter, Manager, State};

use crate::{
    api::claude_ai::ProUsageData,
    storage::database,
    AppState,
};

// ── Data types returned to the frontend ──────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LoginStatus {
    pub is_logged_in: bool,
    pub email: Option<String>,
    pub plan_type: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub refresh_interval_secs: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            refresh_interval_secs: 300,
        }
    }
}

// ── Standard UI commands ──────────────────────────────────────────────────────

#[tauri::command]
pub fn get_usage(state: State<'_, AppState>) -> Result<Option<ProUsageData>, String> {
    let mut usage = state.latest_usage.lock().map_err(|e| e.to_string())?.clone();
    // Merge separately-stored extra_usage if the main response didn't include it.
    if let Some(u) = usage.as_mut() {
        if u.extra_usage.is_none() {
            if let Ok(extra_guard) = state.latest_extra.lock() {
                u.extra_usage = extra_guard.clone();
            }
        }
    }
    Ok(usage)
}

#[tauri::command]
pub fn get_login_status(state: State<'_, AppState>) -> Result<LoginStatus, String> {
    let is_logged_in = *state.is_logged_in.lock().map_err(|e| e.to_string())?;
    let email = state.session_email.lock().map_err(|e| e.to_string())?.clone();
    // Prefer billing-detected plan over usage-derived plan_type.
    let plan_type = state
        .detected_plan
        .lock()
        .ok()
        .and_then(|p| p.clone())
        .or_else(|| {
            state
                .latest_usage
                .lock()
                .ok()
                .and_then(|u| u.as_ref().and_then(|d| d.plan_type.clone()))
        });
    Ok(LoginStatus { is_logged_in, email, plan_type })
}

#[tauri::command]
pub fn open_login_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("session") {
        // Always navigate to the login page so the user sees a clean login form.
        let _ = win.eval("window.location.href = 'https://claude.ai/login';");
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn close_login_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("session") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn trigger_refresh(app: tauri::AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("session")
        .ok_or("Session window not found")?;
    // Pre-fetch account & subscription to capture plan type before usage page loads.
    win.eval(
        "(async function() {\
            try { await fetch('/api/account'); } catch(_) {}\
            try { await fetch('/api/subscription_details'); } catch(_) {}\
            window.location.href = 'https://claude.ai/settings/usage';\
        })();",
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn logout(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    *state.latest_usage.lock().map_err(|e| e.to_string())? = None;
    *state.is_logged_in.lock().map_err(|e| e.to_string())? = false;
    *state.session_email.lock().map_err(|e| e.to_string())? = None;

    if let Some(win) = app.get_webview_window("session") {
        let _ = win.eval(
            "fetch('/api/auth/logout', { method: 'POST' }).finally(() => { window.location.href = 'https://claude.ai/login'; });",
        );
    }

    let _ = app.emit("login-status-changed", false);
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let interval = database::get_setting(&db, "refresh_interval_secs")
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300u64);
    Ok(AppSettings { refresh_interval_secs: interval })
}

#[tauri::command]
pub fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    database::set_setting(&db, "refresh_interval_secs", &settings.refresh_interval_secs.to_string())
        .map_err(|e| e.to_string())
}

// ── Commands called directly from the session webview's JS ────────────────────
// Using direct command invocation instead of plugin:event|emit because
// plugin events from external webviews are unreliable in Tauri 2.0.

#[tauri::command]
pub fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    // Re-use the window if it already exists.
    if let Some(win) = app.get_webview_window("settings") {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let app2 = app.clone();
    app.run_on_main_thread(move || {
        match tauri::WebviewWindowBuilder::new(
            &app2,
            "settings",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("Settings")
        .inner_size(400.0, 560.0)
        .resizable(true)
        .build()
        {
            Ok(_) => log::info!("open_settings_window: created"),
            Err(e) => log::error!("open_settings_window: failed: {}", e),
        }
    })
    .map_err(|e| format!("{e}"))
}

/// Called by the JS URL monitor every 500 ms when the URL changes.
/// Detects login state changes including SPA navigation (history.pushState)
/// which does not trigger the Rust on_page_load callback.
#[tauri::command]
pub fn cm_login_check(
    url: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Parse the URL to check the host, not the full string.
    // Google OAuth URLs contain "claude.ai" in query params (app_domain=...),
    // so a string-contains check would give false positives.
    let parsed = url.parse::<tauri::Url>().unwrap_or_else(|_| {
        "https://unknown/".parse().unwrap()
    });
    let host = parsed.host_str().unwrap_or("");
    let path = parsed.path();
    let on_claude = host == "claude.ai" || host.ends_with(".claude.ai");
    let logged_in = on_claude
        && !path.starts_with("/login")
        && path != "/logout"
        && !path.starts_with("/logout");
    let already_on_usage = path.starts_with("/settings/usage");

    let mut guard = state.is_logged_in.lock().map_err(|e| e.to_string())?;
    if *guard != logged_in {
        *guard = logged_in;
        drop(guard);
        let _ = app.emit("login-status-changed", logged_in);
        log::info!("cm_login_check: logged_in={} url={}", logged_in, url);

        if logged_in && !already_on_usage {
            if let Some(session) = app.get_webview_window("session") {
                let _ = session.eval(
                    "window.location.href = 'https://claude.ai/settings/usage';",
                );
            }
        }
    }
    Ok(())
}

/// Called by the JS fetch interceptor when it captures a usage-related API response.
/// Replaces the plugin:event|emit approach which is unreliable in external webviews.
#[tauri::command]
pub fn cm_api_data(
    url: String,
    data: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let data_val: serde_json::Value =
        serde_json::from_str(&data).map_err(|e| e.to_string())?;

    // ── /usage endpoint: extract 5h/7d quota AND extra_usage ─────────────────
    if let Some(usage) = crate::api::claude_ai::parse_usage(&url, &data_val) {
        // Also parse the nested extra_usage object from the same response.
        if let Some(extra) = crate::api::claude_ai::parse_usage_extra(&data_val) {
            log::info!(
                "cm_api_data: extra_usage — enabled={} spent={:.2} limit={:.2} util={:.1}%",
                extra.enabled, extra.spent, extra.limit, extra.percent_used
            );
            *state.latest_extra.lock().map_err(|e| e.to_string())? = Some(extra);
        }

        log::info!(
            "cm_api_data: 5h={:.1}%  7d={:.1}%",
            usage.five_hour.utilization,
            usage.seven_day.utilization
        );

        // Hide the session window now that we have data.
        if let Some(session) = app.get_webview_window("session") {
            let _ = session.hide();
        }

        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let _ = database::save_usage_record(
                &db,
                "session",
                Some(usage.five_hour.utilization),
                Some(&usage.five_hour.resets_at),
                Some(usage.seven_day.utilization),
                Some(&usage.seven_day.resets_at),
            );
        }
        *state.latest_usage.lock().map_err(|e| e.to_string())? = Some(usage.clone());

        let title = format!("Claude {:.0}%", usage.five_hour.utilization);
        if let Some(tray) = app.tray_by_id("main-tray") {
            let _ = tray.set_title(Some(&title));
            let _ = tray.set_tooltip(Some(&format!(
                "Claude Usage Monitor\n5h: {:.0}%  7d: {:.0}%",
                usage.five_hour.utilization,
                usage.seven_day.utilization
            )));
        }

        // Emit with extra_usage merged from latest_extra (balance may be 0 until
        // /prepaid/credits arrives, which triggers another emit shortly after).
        let mut usage_to_emit = usage.clone();
        if let Ok(extra_guard) = state.latest_extra.lock() {
            usage_to_emit.extra_usage = extra_guard.clone();
        }
        let _ = app.emit("usage-updated", &usage_to_emit);
    }

    // ── /prepaid/credits: patch balance + auto_reload into latest_extra ───────
    if url.contains("/prepaid/credits") {
        if let Some((balance, auto_reload)) = crate::api::claude_ai::parse_prepaid_credits(&data_val) {
            log::info!(
                "cm_api_data: prepaid/credits — balance={:.2} auto_reload={}",
                balance, auto_reload
            );
            // Update latest_extra in place; clone the result for the re-emit.
            let updated_extra = {
                let mut guard = state.latest_extra.lock().map_err(|e| e.to_string())?;
                if let Some(ref mut extra) = *guard {
                    extra.balance = balance;
                    extra.auto_reload = auto_reload;
                } else {
                    // /prepaid/credits arrived before /usage — store partial.
                    *guard = Some(crate::api::claude_ai::ExtraUsage {
                        enabled: false, spent: 0.0, limit: 0.0,
                        balance, percent_used: 0.0,
                        resets_at: String::new(), auto_reload,
                    });
                }
                guard.clone()
            }; // lock released

            // Re-emit usage-updated so the frontend gets the real balance.
            if let Some(mut usage) = state.latest_usage.lock().ok().and_then(|g| g.clone()) {
                usage.extra_usage = updated_extra;
                let _ = app.emit("usage-updated", &usage);
            }
        }
    }

    // ── /subscription_details: patch billing-cycle reset date + plan type ───
    if url.contains("/subscription_details") {
        if let Some(date) = data_val.get("next_charge_date").and_then(|v| v.as_str()) {
            let resets_at = format!("{}T00:00:00Z", date);
            log::info!("cm_api_data: subscription_details — next_charge_date={}", date);
            let mut guard = state.latest_extra.lock().map_err(|e| e.to_string())?;
            if let Some(ref mut extra) = *guard {
                extra.resets_at = resets_at;
            }
        }
        // Also try to capture plan type from subscription details
        if let Some(pt) = crate::api::claude_ai::parse_plan_type(&data_val) {
            log::info!("cm_api_data: subscription_details — plan_type={}", pt);
            *state.detected_plan.lock().map_err(|e| e.to_string())? = Some(pt);
        }
    }

    // ── /api/account: capture email and plan type ────────────────────────────
    if url.contains("/api/account") || url.contains("/api/me") {
        if let Some(pt) = crate::api::claude_ai::parse_plan_type(&data_val) {
            log::info!("cm_api_data: account — plan_type={}", pt);
            *state.detected_plan.lock().map_err(|e| e.to_string())? = Some(pt);
        }
        // Also try to capture email
        if let Some(email) = data_val.get("email").and_then(|v| v.as_str()) {
            *state.session_email.lock().map_err(|e| e.to_string())? = Some(email.to_string());
        }
    }

    Ok(())
}

/// Called by the JS window.open() interceptor when claude.ai opens a Google
/// OAuth popup. Creates a real Tauri popup window so the session window stays
/// on claude.ai. The popup has OAUTH_POPUP_JS injected which:
///   - fakes window.opener so postMessage calls are relayed via cm_oauth_message
///   - monitors URL changes via cm_popup_navigated for redirect-based flows
#[tauri::command]
pub fn cm_open_popup(url: String, app: tauri::AppHandle) -> Result<(), String> {
    log::info!("cm_open_popup: creating oauth popup for {}", url);
    let target: tauri::Url = url.parse().map_err(|e| format!("{e}"))?;
    let app2 = app.clone();
    app.run_on_main_thread(move || {
        // Close any stale popup first.
        if let Some(old) = app2.get_webview_window("oauth-popup") {
            let _ = old.close();
        }
        match tauri::WebviewWindowBuilder::new(
            &app2,
            "oauth-popup",
            tauri::WebviewUrl::External(target),
        )
        .initialization_script(crate::OAUTH_POPUP_JS)
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15")
        .title("Sign in with Google")
        .inner_size(500.0, 650.0)
        .build()
        {
            Ok(_) => log::info!("cm_open_popup: popup window created"),
            Err(e) => log::error!("cm_open_popup: failed to create popup: {}", e),
        }
    })
    .map_err(|e| format!("{e}"))
}

/// Called by the oauth popup's URL monitor on every navigation.
/// Handles the server-side redirect OAuth flow: when the popup lands back on
/// claude.ai proper (not on a login/auth/api page) the OAuth exchange is done.
#[tauri::command]
pub fn cm_popup_navigated(url: String, app: tauri::AppHandle) -> Result<(), String> {
    // Bail early if the popup was already closed (e.g. by cm_oauth_message).
    if app.get_webview_window("oauth-popup").is_none() {
        return Ok(());
    }

    let parsed = url.parse::<tauri::Url>().unwrap_or_else(|_| "https://unknown/".parse().unwrap());
    let host = parsed.host_str().unwrap_or("");
    let path = parsed.path();
    let on_claude = host == "claude.ai" || host.ends_with(".claude.ai");

    // OAuth is done when the popup lands on claude.ai but NOT on a login,
    // auth-callback, or API-callback path.
    let done = on_claude
        && !path.starts_with("/login")
        && !path.starts_with("/auth")
        && !path.starts_with("/api/");

    if done {
        log::info!("cm_popup_navigated: redirect-flow OAuth complete at {}", url);
        if let Some(popup) = app.get_webview_window("oauth-popup") {
            let _ = popup.close();
        }
        if let Some(session) = app.get_webview_window("session") {
            let _ = session.eval("window.location.href = 'https://claude.ai/settings/usage';");
        }
    } else {
        log::debug!("cm_popup_navigated: url={}", url);
    }
    Ok(())
}

/// Called by the oauth popup's fake window.opener.postMessage relay.
/// Dispatches a MessageEvent on the session window so claude.ai can complete
/// the postMessage-based OAuth flow without a real window.opener.
#[tauri::command]
pub fn cm_oauth_message(data: String, origin: String, app: tauri::AppHandle) -> Result<(), String> {
    let preview = &data[..data.len().min(120)];
    log::info!("cm_oauth_message: origin={} data_preview={}", origin, preview);

    if let Some(session) = app.get_webview_window("session") {
        // 1. Dispatch the MessageEvent so claude.ai's GIS listener can process
        //    the auth token.  We use JSON.parse() to safely re-hydrate the value
        //    from the JSON string, avoiding any JS literal escaping issues.
        let safe_origin = if origin == "*" { "https://claude.ai".to_string() } else { origin.clone() };
        let json_data_literal = serde_json::to_string(&data).unwrap_or_else(|_| "null".to_string());
        let dispatch_js = format!(
            r#"(function(){{
                try {{
                    var d = JSON.parse({json});
                    window.dispatchEvent(new MessageEvent('message',{{
                        data: d,
                        origin: '{origin}',
                        lastEventId: ''
                    }}));
                }} catch(e) {{ console.warn('cm_oauth_message dispatch:', e); }}
            }})();"#,
            json = json_data_literal,
            origin = safe_origin,
        );
        let _ = session.eval(&dispatch_js);

        // 2. Mark the mock popup object as closed so claude.ai's polling loop
        //    (if (popup.closed) clearInterval(...)}) can advance past this point.
        let _ = session.eval(
            "if (window.__cm_oauth_mock) { window.__cm_oauth_mock.closed = true; }"
        );

        // 3. Fallback: if the GIS handler can't process the relay (e.g. because
        //    event.source is null), give it 2 s then navigate the session window
        //    to /settings/usage directly.  If the exchange already succeeded this
        //    is a no-op; if it failed this recovers the UI from spinning.
        let _ = session.eval(
            "setTimeout(function(){\
                if (window.location.pathname.startsWith('/login') || window.location.pathname === '/') {\
                    window.location.href = 'https://claude.ai/settings/usage';\
                }\
            }, 2000);"
        );
    }

    if let Some(popup) = app.get_webview_window("oauth-popup") {
        let _ = popup.close();
    }
    Ok(())
}
