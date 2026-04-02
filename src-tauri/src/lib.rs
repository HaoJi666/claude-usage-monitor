use std::sync::Mutex;
use std::time::Duration;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

pub mod api;
pub mod commands;
pub mod storage;

use storage::database;

/// Injected into the OAuth popup window.
/// - Fakes window.opener so postMessage calls from claude.ai/auth/callback
///   are relayed back to the session window via cm_oauth_message.
/// - URL monitor calls cm_popup_navigated on every URL change so Rust knows
///   when the OAuth redirect flow completes (popup lands on claude.ai proper).
pub const OAUTH_POPUP_JS: &str = r#"
(function () {
  if (window.__cm_popup_injected) return;
  window.__cm_popup_injected = true;

  // ── Fake window.opener ────────────────────────────────────────────────
  // The real window.opener is null in a Tauri popup window.
  // claude.ai's /auth/callback page calls window.opener.postMessage(token)
  // to hand the auth result back to the main claude.ai tab.
  // We intercept that call and forward it to the session window via Tauri IPC.
  try {
    Object.defineProperty(window, 'opener', {
      configurable: true,
      get: function () {
        return {
          postMessage: function (msg, targetOrigin) {
            try {
              var ipc = window.__TAURI_INTERNALS__;
              if (ipc && ipc.invoke) {
                // Use the actual sender origin (e.g. accounts.google.com) so
                // that event.origin in the session window matches what claude.ai
                // expects from its GIS message handler.
                var senderOrigin = (window.location.origin && window.location.origin !== 'null')
                  ? window.location.origin
                  : (window.location.protocol + '//' + window.location.host);
                ipc.invoke('cm_oauth_message', {
                  data: JSON.stringify(msg),
                  origin: senderOrigin
                });
              }
            } catch (_) {}
          },
          closed: false,
          close: function () {},
          focus: function () {},
          location: { href: 'https://claude.ai/' }
        };
      }
    });
  } catch (_) {}

  // ── URL monitor: detect redirect-based OAuth completion ───────────────
  var _lastUrl = '';
  function cm_popup_check_url() {
    var url = window.location.href;
    if (url !== _lastUrl) {
      _lastUrl = url;
      try {
        var ipc = window.__TAURI_INTERNALS__;
        if (ipc && ipc.invoke) {
          ipc.invoke('cm_popup_navigated', { url: url });
        }
      } catch (_) {}
    }
  }
  setTimeout(cm_popup_check_url, 300);
  setInterval(cm_popup_check_url, 500);
})();
"#;

/// Injected into the session webview on every page load (document start).
///
/// Key design decisions:
/// - Uses `ipc.invoke('command_name', args)` to call Tauri commands directly.
///   The old `plugin:event|emit` approach was unreliable in external webviews.
/// - Monitors URL changes via setInterval to catch SPA navigation
///   (history.pushState / popstate) that doesn't trigger on_page_load in Rust.
const FETCH_INTERCEPTOR_JS: &str = r#"
(function () {
  'use strict';
  if (window.__cm_injected) return;
  window.__cm_injected = true;

  // ── fetch() interceptor: capture usage API responses ─────────────────
  var _orig = window.fetch;
  window.fetch = async function () {
    var resp;
    try { resp = await _orig.apply(this, arguments); } catch (e) { throw e; }

    try {
      var u = arguments[0];
      u = typeof u === 'string' ? u : (u && u.url ? u.url : String(u || ''));

      // No domain check — session window only loads claude.ai, and
      // internal API calls use relative paths like /api/usage (no domain).
      var hit = (
        u.indexOf('/api/usage')          !== -1 ||
        u.indexOf('/api/organizations')  !== -1 ||
        u.indexOf('/api/account')        !== -1 ||
        u.indexOf('usage_limit')         !== -1 ||
        u.indexOf('rate_limit')          !== -1
      );

      if (resp.ok && hit) {
        resp.clone().json().then(function (d) {
          try {
            var ipc = window.__TAURI_INTERNALS__;
            if (ipc && ipc.invoke) {
              // Direct command call — more reliable than plugin:event|emit
              // in external (non-localhost) webviews.
              ipc.invoke('cm_api_data', { url: u, data: JSON.stringify(d) });
            }
          } catch (_) {}
        }).catch(function () {});
      }
    } catch (_) {}

    return resp;
  };

  // ── window.open() interceptor: handle OAuth popups via Tauri ─────────
  // Only intercept on claude.ai itself. When the session window navigates
  // to accounts.google.com for OAuth, we must NOT intercept Google's own
  // window.open() calls or we'd trigger cm_open_popup recursively.
  var _origOpen = window.open;
  window.__cm_oauth_mock = null;
  window.open = function (url, target, features) {
    var onClaude = window.location.hostname === 'claude.ai' ||
                   window.location.hostname.endsWith('.claude.ai');
    if (!onClaude) {
      return _origOpen ? _origOpen.apply(this, arguments) : null;
    }
    try {
      var ipc = window.__TAURI_INTERNALS__;
      if (ipc && ipc.invoke && url) {
        ipc.invoke('cm_open_popup', { url: String(url) });
        // Store a reference so cm_oauth_message can flip closed=true later.
        var mock = { closed: false, close: function () {}, focus: function () {}, postMessage: function () {} };
        window.__cm_oauth_mock = mock;
        return mock;
      }
    } catch (_) {}
    return _origOpen ? _origOpen.apply(this, arguments) : null;
  };

  // ── SPA URL change monitor ─────────────────────────────────────────────
  // claude.ai is a Next.js SPA. Login / logout navigation uses
  // history.pushState and does NOT trigger on_page_load in Rust.
  // We poll here and report every URL change via a direct command call.
  var cm_last_url = '';
  function cm_report_url() {
    var url = window.location.href;
    if (url !== cm_last_url) {
      cm_last_url = url;
      try {
        var ipc = window.__TAURI_INTERNALS__;
        if (ipc && ipc.invoke) {
          ipc.invoke('cm_login_check', { url: url });
        }
      } catch (_) {}
    }
  }
  // Report immediately (slight delay lets the SPA settle) and every 500 ms.
  setTimeout(cm_report_url, 300);
  setInterval(cm_report_url, 500);
})();
"#;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub http_client: reqwest::Client,
    pub latest_usage: Mutex<Option<api::claude_ai::ProUsageData>>,
    pub is_logged_in: Mutex<bool>,
    pub session_email: Mutex<Option<String>>,
}

fn get_db_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    let data_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir");
    std::fs::create_dir_all(&data_dir).expect("Failed to create app data dir");
    data_dir.join("usage.db")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // ── macOS: hide from Dock, appear only in menu bar ────────
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // ── Database ──────────────────────────────────────────────
            let db_path = get_db_path(app.handle());
            let conn = rusqlite::Connection::open(&db_path)
                .expect("Failed to open database");
            database::initialize(&conn).expect("Failed to initialize database");

            let http_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client");

            app.manage(AppState {
                db: Mutex::new(conn),
                http_client,
                latest_usage: Mutex::new(None),
                is_logged_in: Mutex::new(false),
                session_email: Mutex::new(None),
            });

            // ── Session webview (login + data capture) ────────────────
            // on_page_load handles full-page navigations (e.g. app restart
            // when already logged in, or direct window.location.href changes
            // from Rust). SPA navigation is handled by cm_login_check command.
            let app_nav = app.handle().clone();
            tauri::WebviewWindowBuilder::new(
                app,
                "session",
                tauri::WebviewUrl::External(
                    "https://claude.ai/login".parse().expect("invalid URL"),
                ),
            )
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15")
            .initialization_script(FETCH_INTERCEPTOR_JS)
            .title("Claude – Login")
            .inner_size(480.0, 720.0)
            .min_inner_size(360.0, 600.0)
            .visible(false)
            .on_page_load(move |win, payload| {
                if payload.event() != tauri::webview::PageLoadEvent::Finished {
                    return;
                }
                let url_parsed = payload.url();
                let host = url_parsed.host_str().unwrap_or("");
                let path = url_parsed.path();
                // Use the HOST to determine if we are on claude.ai.
                // A string-contains check is wrong because Google OAuth URLs
                // embed claude.ai as a query parameter (app_domain=...).
                let on_claude = host == "claude.ai" || host.ends_with(".claude.ai");
                let logged_in = on_claude
                    && !path.starts_with("/login")
                    && path != "/logout"
                    && !path.starts_with("/logout");
                let already_on_usage = path.starts_with("/settings/usage");

                log::info!("on_page_load: host={} path={} logged_in={}", host, path, logged_in);

                if let Some(state) = app_nav.try_state::<AppState>() {
                    let mut guard = state.is_logged_in.lock().unwrap();
                    if *guard != logged_in {
                        *guard = logged_in;
                        drop(guard);
                        let _ = app_nav.emit("login-status-changed", logged_in);

                        if logged_in && !already_on_usage {
                            let _ = win.eval(
                                "window.location.href = 'https://claude.ai/settings/usage';",
                            );
                        }
                    }
                }
            })
            .build()?;

            // ── Tray icon ─────────────────────────────────────────────
            // Right-click context menu with Exit option.
            let quit_item = MenuItem::with_id(app, "quit", "Exit App", true, None::<&str>)?;
            let tray_menu = Menu::new(app)?;
            tray_menu.append(&quit_item)?;

            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .title("Claude")
                .tooltip("Claude Usage Monitor")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let cx = position.x as i32;
                                let cy = position.y as i32;
                                let _ = window.set_position(tauri::PhysicalPosition::new(
                                    (cx - 180).max(0),
                                    cy + 4,
                                ));
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // ── Background refresh task ────────────────────────────────
            {
                let app_bg = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    loop {
                        let interval_secs = app_bg
                            .try_state::<AppState>()
                            .map(|s| {
                                let db = s.db.lock().unwrap();
                                database::get_setting(&db, "refresh_interval_secs")
                                    .ok()
                                    .flatten()
                                    .and_then(|v| v.parse().ok())
                                    .unwrap_or(300u64)
                            })
                            .unwrap_or(300u64);

                        let logged_in = app_bg
                            .try_state::<AppState>()
                            .map(|s| *s.is_logged_in.lock().unwrap())
                            .unwrap_or(false);

                        if logged_in {
                            if let Some(session) = app_bg.get_webview_window("session") {
                                let _ = session.eval(
                                    "window.location.href = 'https://claude.ai/settings/usage';",
                                );
                            }
                        }

                        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_usage,
            commands::get_login_status,
            commands::open_login_window,
            commands::close_login_window,
            commands::trigger_refresh,
            commands::logout,
            commands::get_settings,
            commands::save_settings,
            // Called directly from the session webview's JS scripts:
            commands::cm_login_check,
            commands::cm_api_data,
            commands::cm_open_popup,
            // Called from the oauth popup window's JS:
            commands::cm_popup_navigated,
            commands::cm_oauth_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
